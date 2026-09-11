//! Windows audio device metadata boundary for M02.
//!
//! The adapter keeps endpoint enumeration and process/session discovery
//! read-only, while its explicit `SharedCapture` and `SharedRender` clients
//! provide bounded, caller-owned shared-mode stream I/O for the native audio
//! milestone. Streams are never opened implicitly by metadata discovery; a
//! caller must explicitly select an endpoint and invoke the stream methods.

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use windows_core::Interface;

/// Maximum number of OS-provided audio-session names retained per process.
pub const MAX_APPLICATION_AUDIO_DISPLAY_NAMES: usize = 64;
/// Maximum UTF-8 byte length of one retained OS-provided session name.
pub const MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES: usize = 256;
/// Maximum number of process identities retained by application discovery.
pub const MAX_APPLICATIONS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointDirection {
    Capture,
    Render,
}

/// Windows default-device roles that can be selected by an explicit
/// follow-default binding. The role is kept separate from the endpoint ID so
/// a default-device change can be observed without silently replacing a
/// pinned endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultEndpointRole {
    Console,
    Multimedia,
    Communications,
}

impl DefaultEndpointRole {
    pub const ALL: [Self; 3] = [Self::Console, Self::Multimedia, Self::Communications];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Console => "console",
            Self::Multimedia => "multimedia",
            Self::Communications => "communications",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefaultEndpointBinding {
    pub direction: EndpointDirection,
    pub role: DefaultEndpointRole,
    pub endpoint_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointDisplayInfo {
    pub id: String,
    pub direction: EndpointDirection,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EndpointState {
    Active,
    Disabled,
    Unplugged,
    NotPresent,
    Unknown(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointStateInfo {
    pub id: String,
    pub direction: EndpointDirection,
    pub state: EndpointState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EndpointStateChange {
    Added(EndpointStateInfo),
    Removed(EndpointStateInfo),
    Changed {
        before: EndpointStateInfo,
        after: EndpointStateInfo,
    },
}

fn endpoint_state_from_raw(raw_state: u32) -> EndpointState {
    match raw_state {
        1 => EndpointState::Active,
        2 => EndpointState::Disabled,
        8 => EndpointState::Unplugged,
        4 => EndpointState::NotPresent,
        value => EndpointState::Unknown(value),
    }
}

#[cfg(windows)]
const IOCTL_AUDIOROUTER_BRIDGE_OPEN: u32 = (0x22 << 16) | (0x800 << 2) | (3 << 14) | 0x3;
#[cfg(windows)]
const IOCTL_AUDIOROUTER_BRIDGE_CLOSE: u32 = (0x22 << 16) | (0x801 << 2) | (3 << 14) | 0x3;
#[cfg(windows)]
const IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT: u32 = (0x22 << 16) | (0x802 << 2) | (3 << 14) | 0x3;

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct NativeBridgeOpenRequest {
    protocol_major: u16,
    protocol_minor: u16,
    bus_id_bytes: u16,
    channels: u16,
    frames_per_quantum: u16,
    direction: u16,
    sample_rate_hz: u32,
    lease_ms: u32,
    generation: u64,
    section_handle: u64,
    mapping_bytes: u32,
    reserved2: u32,
    bus_id: [u16; 64],
}

#[cfg(windows)]
pub struct NativeBridgeControlClient {
    handle: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
pub struct NativeBridgeSectionHandle {
    file: windows::Win32::Foundation::HANDLE,
    section: windows::Win32::Foundation::HANDLE,
    mapping_bytes: u32,
}

#[cfg(windows)]
impl NativeBridgeSectionHandle {
    /// Creates a temporary-section handle over an already-sized bridge file.
    /// The caller must keep this handle alive while the driver lease is active.
    pub fn for_file(
        path: impl AsRef<std::path::Path>,
        mapping_bytes: u32,
    ) -> Result<Self, windows::core::Error> {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows::Win32::System::Memory::{CreateFileMappingW, PAGE_READWRITE};
        if mapping_bytes == 0 {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "invalid mapping size",
            ));
        }
        let path = path.as_ref().to_string_lossy();
        let mut wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let file = unsafe {
            CreateFileW(
                PCWSTR(wide.as_mut_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?
        };
        let size_high = 0;
        let section = unsafe {
            match CreateFileMappingW(
                file,
                None,
                PAGE_READWRITE,
                size_high,
                mapping_bytes,
                PCWSTR::null(),
            ) {
                Ok(section) => section,
                Err(error) => {
                    let _ = windows::Win32::Foundation::CloseHandle(file);
                    return Err(error);
                }
            }
        };
        Ok(Self {
            file,
            section,
            mapping_bytes,
        })
    }

    pub fn raw_handle(&self) -> u64 {
        self.section.0 as usize as u64
    }

    pub fn mapping_bytes(&self) -> u32 {
        self.mapping_bytes
    }

    pub fn read_bytes(&self, offset: u32, length: u32) -> Result<Vec<u8>, windows::core::Error> {
        use windows::Win32::System::Memory::{MapViewOfFile, UnmapViewOfFile, FILE_MAP_READ};
        let end = offset.checked_add(length).ok_or_else(|| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "mapping range overflow",
            )
        })?;
        if end > self.mapping_bytes {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "mapping range outside section",
            ));
        }
        let view = unsafe {
            MapViewOfFile(
                self.section,
                FILE_MAP_READ,
                0,
                0,
                self.mapping_bytes as usize,
            )
        };
        if view.Value.is_null() {
            return Err(windows::core::Error::from_thread());
        }
        let mut output = vec![0_u8; length as usize];
        // SAFETY: the view is mapped for `mapping_bytes`, and the checked
        // range is within that view. The destination owns `length` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(
                (view.Value as *const u8).add(offset as usize),
                output.as_mut_ptr(),
                length as usize,
            );
            let _ = UnmapViewOfFile(view);
        }
        Ok(output)
    }
}

#[cfg(windows)]
impl Drop for NativeBridgeSectionHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.section);
            let _ = windows::Win32::Foundation::CloseHandle(self.file);
        }
    }
}

#[cfg(windows)]
#[derive(Debug)]
pub enum NativeBridgeControllerError {
    InvalidDuplex,
    Windows(windows::core::Error),
    Session(NativeBridgeSessionError),
}

#[cfg(windows)]
#[derive(Debug)]
pub enum NativeBridgeOutputWorkerError {
    Bridge(NativeBridgeControllerError),
    Audio(AudioError),
}

#[cfg(windows)]
#[derive(Debug)]
pub enum NativeBridgeInputWorkerError {
    Bridge(NativeBridgeControllerError),
    Audio(AudioError),
}

#[cfg(windows)]
#[derive(Debug)]
enum NativeBridgeInputPumpError {
    Bridge(NativeBridgeControllerError),
    Audio(AudioError),
}

#[cfg(windows)]
fn native_bridge_input_error_is_silence(error: &NativeBridgeControllerError) -> bool {
    matches!(
        error,
        NativeBridgeControllerError::Session(NativeBridgeSessionError::Region(
            NativeBridgeRegionError::Empty
                | NativeBridgeRegionError::Busy
                | NativeBridgeRegionError::TornRead
                | NativeBridgeRegionError::SequenceRegression
                | NativeBridgeRegionError::StaleGeneration,
        ))
    )
}

#[cfg(windows)]
/// Consumes the negotiated virtual render-source bridge and renders its
/// processed output to one explicitly selected physical endpoint.
pub struct NativeBridgeInputWorker {
    source: NativeBridgeController,
    render: SharedRender,
    bridge: WasapiSchedulerBridge,
    last_sequence: u64,
    running: bool,
}

#[cfg(windows)]
impl NativeBridgeInputWorker {
    pub fn new(
        source: NativeBridgeController,
        render: SharedRender,
        bridge: WasapiSchedulerBridge,
    ) -> Self {
        Self {
            source,
            render,
            bridge,
            last_sequence: 0,
            running: false,
        }
    }

    pub fn start(&mut self) -> Result<(), NativeBridgeInputWorkerError> {
        if self.running {
            return Ok(());
        }
        self.render
            .start()
            .map_err(NativeBridgeInputWorkerError::Audio)?;
        self.running = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), NativeBridgeInputWorkerError> {
        self.running = false;
        self.bridge
            .reset_stream()
            .map_err(NativeBridgeInputWorkerError::Audio)?;
        self.render
            .stop()
            .map_err(NativeBridgeInputWorkerError::Audio)
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn heartbeat(&mut self) -> Result<(), NativeBridgeInputWorkerError> {
        self.source
            .heartbeat()
            .map_err(NativeBridgeInputWorkerError::Bridge)
    }

    /// Pump one bridge block without waiting. Empty, busy, torn, stale, or
    /// repeated bridge slots are rendered as a fresh silent quantum; no old
    /// bridge payload is replayed after a reconnect or owner handoff.
    pub fn pump(&mut self) -> Result<WasapiSchedulerPump, NativeBridgeInputWorkerError> {
        if !self.running {
            return Err(NativeBridgeInputWorkerError::Audio(
                AudioError::ProcessingStateUnavailable,
            ));
        }
        self.bridge
            .pump_from_native_render_source(&self.source, &self.render, &mut self.last_sequence)
            .map_err(|error| match error {
                NativeBridgeInputPumpError::Bridge(error) => {
                    NativeBridgeInputWorkerError::Bridge(error)
                }
                NativeBridgeInputPumpError::Audio(error) => {
                    NativeBridgeInputWorkerError::Audio(error)
                }
            })
    }
}

#[cfg(windows)]
/// Couples an already-prepared endpoint worker to one negotiated virtual
/// capture sink. Construction is deliberately stopped-by-default: endpoint
/// activation, driver installation, and default-device changes remain outside
/// this type. The bridge writer is an independent mapped view, so heartbeat
/// and lease ownership stay with the controller while the audio callback only
/// copies into its preallocated slot.
pub struct NativeBridgeOutputWorker {
    endpoint: WasapiEndpointWorker,
    controller: NativeBridgeController,
    writer: NativeBridgeRealtimeWriter,
}

#[cfg(windows)]
impl NativeBridgeOutputWorker {
    pub fn new(
        endpoint: WasapiEndpointWorker,
        controller: NativeBridgeController,
    ) -> Result<Self, NativeBridgeOutputWorkerError> {
        let writer = controller
            .realtime_writer()
            .map_err(NativeBridgeOutputWorkerError::Bridge)?;
        Ok(Self {
            endpoint,
            controller,
            writer,
        })
    }

    pub fn is_running(&self) -> bool {
        self.endpoint.is_running()
    }

    pub fn published_blocks(&self) -> u64 {
        self.writer.published_blocks()
    }

    pub fn start(&mut self) -> Result<(), NativeBridgeOutputWorkerError> {
        self.endpoint
            .start()
            .map_err(NativeBridgeOutputWorkerError::Audio)
    }

    pub fn stop(&mut self) -> Result<(), NativeBridgeOutputWorkerError> {
        self.endpoint
            .stop()
            .map_err(NativeBridgeOutputWorkerError::Audio)
    }

    /// Refresh the kernel lease from the control/worker thread. This is not
    /// called from the realtime callback.
    pub fn heartbeat(&mut self) -> Result<(), NativeBridgeOutputWorkerError> {
        self.controller
            .heartbeat()
            .map_err(NativeBridgeOutputWorkerError::Bridge)
    }

    pub fn pump_available(
        &mut self,
        max_packets: u32,
    ) -> Result<WasapiSchedulerPump, NativeBridgeOutputWorkerError> {
        self.endpoint
            .pump_available_with_tap(max_packets, &self.writer)
            .map_err(NativeBridgeOutputWorkerError::Audio)
    }

    pub fn endpoint(&self) -> &WasapiEndpointWorker {
        &self.endpoint
    }
}

#[cfg(windows)]
/// Explicit owner of the driver lease and its broker-side mapped session.
///
/// Construction claims the kernel lease before callers can publish blocks.
/// The mapping is still broker-owned until the driver receives a future section
/// handle; this type deliberately does not claim kernel access to the file view.
pub struct NativeBridgeController {
    client: NativeBridgeControlClient,
    session: NativeBridgeSession,
    section: Option<NativeBridgeSectionHandle>,
}

#[cfg(windows)]
/// Owns the two directional bridge leases for one virtual bus.
///
/// The render-source side is read by the driver-facing consumer and the
/// capture-sink side is written by an engine `AudioTap`. They use separate
/// mappings and lease slots, so a bus cannot accidentally overwrite its own
/// opposite direction.
pub struct NativeBridgeDuplexController {
    render: NativeBridgeController,
    capture: NativeBridgeController,
}

#[cfg(windows)]
impl NativeBridgeDuplexController {
    fn validate_hellos(
        render_hello: &audiorouter_protocol::AudioBridgeHello,
        capture_hello: &audiorouter_protocol::AudioBridgeHello,
    ) -> Result<(), NativeBridgeControllerError> {
        if render_hello.direction != audiorouter_protocol::AudioBridgeDirection::RenderSource
            || capture_hello.direction != audiorouter_protocol::AudioBridgeDirection::CaptureSink
            || render_hello.bus_id != capture_hello.bus_id
        {
            return Err(NativeBridgeControllerError::InvalidDuplex);
        }
        Ok(())
    }

    pub fn create(
        device_path: &str,
        render_mapping_path: impl AsRef<std::path::Path>,
        capture_mapping_path: impl AsRef<std::path::Path>,
        render_hello: audiorouter_protocol::AudioBridgeHello,
        capture_hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<Self, NativeBridgeControllerError> {
        Self::validate_hellos(&render_hello, &capture_hello)?;
        let render =
            NativeBridgeController::create(device_path, render_mapping_path, render_hello)?;
        match NativeBridgeController::create(device_path, capture_mapping_path, capture_hello) {
            Ok(capture) => Ok(Self { render, capture }),
            Err(error) => {
                drop(render);
                Err(error)
            }
        }
    }

    /// Claim both directional leases with broker-created sections. The child
    /// controllers retain their section handles through heartbeat and close.
    pub fn create_with_sections(
        device_path: &str,
        render_mapping_path: impl AsRef<std::path::Path>,
        capture_mapping_path: impl AsRef<std::path::Path>,
        render_hello: audiorouter_protocol::AudioBridgeHello,
        capture_hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<Self, NativeBridgeControllerError> {
        Self::validate_hellos(&render_hello, &capture_hello)?;
        let render = NativeBridgeController::create_with_section(
            device_path,
            render_mapping_path,
            render_hello,
        )?;
        match NativeBridgeController::create_with_section(
            device_path,
            capture_mapping_path,
            capture_hello,
        ) {
            Ok(capture) => Ok(Self { render, capture }),
            Err(error) => {
                drop(render);
                Err(error)
            }
        }
    }

    pub fn heartbeat(&mut self) -> Result<(), NativeBridgeControllerError> {
        self.render.heartbeat()?;
        self.capture.heartbeat()
    }

    pub fn render_read_into(
        &self,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeControllerError> {
        self.render.read_into(samples)
    }

    pub fn render_read_into_after(
        &self,
        minimum_sequence: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeControllerError> {
        self.render.read_into_after(minimum_sequence, samples)
    }

    pub fn capture_writer(
        &self,
    ) -> Result<NativeBridgeRealtimeWriter, NativeBridgeControllerError> {
        self.capture.realtime_writer()
    }

    pub fn close(self) -> Result<(), NativeBridgeControllerError> {
        let NativeBridgeDuplexController { render, capture } = self;
        capture.close()?;
        render.close()
    }
}

#[cfg(windows)]
impl NativeBridgeController {
    pub fn create(
        device_path: &str,
        mapping_path: impl AsRef<std::path::Path>,
        hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<Self, NativeBridgeControllerError> {
        let client = NativeBridgeControlClient::open(device_path)
            .map_err(NativeBridgeControllerError::Windows)?;
        let session = NativeBridgeSession::create(mapping_path, hello.clone())
            .map_err(NativeBridgeControllerError::Session)?;
        client
            .open_bridge(&hello)
            .map_err(NativeBridgeControllerError::Windows)?;
        Ok(Self {
            client,
            session,
            section: None,
        })
    }

    pub fn create_with_section(
        device_path: &str,
        mapping_path: impl AsRef<std::path::Path>,
        hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<Self, NativeBridgeControllerError> {
        let client = NativeBridgeControlClient::open(device_path)
            .map_err(NativeBridgeControllerError::Windows)?;
        let session = NativeBridgeSession::create(&mapping_path, hello.clone())
            .map_err(NativeBridgeControllerError::Session)?;
        let section =
            NativeBridgeSectionHandle::for_file(mapping_path, session.mapping_bytes() as u32)
                .map_err(NativeBridgeControllerError::Windows)?;
        client
            .open_bridge_with_mapping(&hello, section.raw_handle(), section.mapping_bytes())
            .map_err(NativeBridgeControllerError::Windows)?;
        Ok(Self {
            client,
            session,
            section: Some(section),
        })
    }

    pub fn heartbeat(&mut self) -> Result<(), NativeBridgeControllerError> {
        if let Some(section) = &self.section {
            self.client
                .heartbeat_with_mapping(
                    self.session.hello(),
                    section.raw_handle(),
                    section.mapping_bytes(),
                )
                .map_err(NativeBridgeControllerError::Windows)?;
        } else {
            self.client
                .heartbeat(self.session.hello())
                .map_err(NativeBridgeControllerError::Windows)?;
        }
        self.session
            .heartbeat()
            .map_err(NativeBridgeControllerError::Session)
    }

    pub fn write(&mut self, samples: &[f32]) -> Result<u64, NativeBridgeControllerError> {
        self.session
            .write(samples)
            .map_err(NativeBridgeControllerError::Session)
    }

    pub fn read_into(
        &self,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeControllerError> {
        self.session
            .read_into(samples)
            .map_err(NativeBridgeControllerError::Session)
    }

    pub fn read_into_after(
        &self,
        minimum_sequence: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeControllerError> {
        self.session
            .read_into_after(minimum_sequence, samples)
            .map_err(NativeBridgeControllerError::Session)
    }

    /// Create the realtime producer view for this negotiated bridge. The
    /// returned writer owns an independent file mapping and may be shared as
    /// an `AudioTap`; lease heartbeat and close remain owned by this
    /// controller.
    pub fn realtime_writer(
        &self,
    ) -> Result<NativeBridgeRealtimeWriter, NativeBridgeControllerError> {
        self.session
            .realtime_writer()
            .map_err(NativeBridgeControllerError::Session)
    }

    pub fn close(mut self) -> Result<(), NativeBridgeControllerError> {
        if let Some(section) = &self.section {
            self.client
                .close_with_mapping(
                    self.session.hello(),
                    section.raw_handle(),
                    section.mapping_bytes(),
                )
                .map_err(NativeBridgeControllerError::Windows)?;
        } else {
            self.client
                .close(self.session.hello())
                .map_err(NativeBridgeControllerError::Windows)?;
        }
        self.session
            .flush()
            .map_err(NativeBridgeControllerError::Session)
    }
}

#[cfg(windows)]
impl Drop for NativeBridgeController {
    fn drop(&mut self) {
        // Best-effort release for panic/error paths. The kernel lease timeout
        // remains the final recovery boundary when close cannot be delivered.
        if let Some(section) = &self.section {
            let _ = self.client.close_with_mapping(
                self.session.hello(),
                section.raw_handle(),
                section.mapping_bytes(),
            );
        } else {
            let _ = self.client.close(self.session.hello());
        }
    }
}

#[cfg(windows)]
trait NativeRenderSource {
    fn read_into_after(
        &self,
        minimum_sequence: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeControllerError>;
}

#[cfg(windows)]
impl NativeRenderSource for NativeBridgeController {
    fn read_into_after(
        &self,
        minimum_sequence: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeControllerError> {
        NativeBridgeController::read_into_after(self, minimum_sequence, samples)
    }
}

#[cfg(windows)]
impl NativeBridgeControlClient {
    /// Opens the secured driver endpoint explicitly; discovery never opens it.
    pub fn open(path: &str) -> Result<Self, windows::core::Error> {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE};
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, OPEN_EXISTING,
        };
        let mut wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let handle = unsafe {
            CreateFileW(
                PCWSTR(wide.as_mut_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?
        };
        Ok(Self { handle })
    }

    pub fn open_bridge(
        &self,
        hello: &audiorouter_protocol::AudioBridgeHello,
    ) -> Result<(), windows::core::Error> {
        let request = native_bridge_open_request(hello, 0, 0).map_err(|_| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "invalid hello",
            )
        })?;
        self.ioctl(IOCTL_AUDIOROUTER_BRIDGE_OPEN, &request)
    }

    pub fn open_bridge_with_mapping(
        &self,
        hello: &audiorouter_protocol::AudioBridgeHello,
        section_handle: u64,
        mapping_bytes: u32,
    ) -> Result<(), windows::core::Error> {
        let request =
            native_bridge_open_request(hello, section_handle, mapping_bytes).map_err(|_| {
                windows::core::Error::new(
                    windows::core::HRESULT(0x80070057u32 as i32),
                    "invalid hello or mapping",
                )
            })?;
        self.ioctl(IOCTL_AUDIOROUTER_BRIDGE_OPEN, &request)
    }

    pub fn heartbeat(
        &self,
        hello: &audiorouter_protocol::AudioBridgeHello,
    ) -> Result<(), windows::core::Error> {
        let request = native_bridge_open_request(hello, 0, 0).map_err(|_| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "invalid hello",
            )
        })?;
        self.ioctl(IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT, &request)
    }

    pub fn heartbeat_with_mapping(
        &self,
        hello: &audiorouter_protocol::AudioBridgeHello,
        section_handle: u64,
        mapping_bytes: u32,
    ) -> Result<(), windows::core::Error> {
        let request =
            native_bridge_open_request(hello, section_handle, mapping_bytes).map_err(|_| {
                windows::core::Error::new(
                    windows::core::HRESULT(0x80070057u32 as i32),
                    "invalid hello or mapping",
                )
            })?;
        self.ioctl(IOCTL_AUDIOROUTER_BRIDGE_HEARTBEAT, &request)
    }

    pub fn close(
        &self,
        hello: &audiorouter_protocol::AudioBridgeHello,
    ) -> Result<(), windows::core::Error> {
        let request = native_bridge_open_request(hello, 0, 0).map_err(|_| {
            windows::core::Error::new(
                windows::core::HRESULT(0x80070057u32 as i32),
                "invalid hello",
            )
        })?;
        self.ioctl(IOCTL_AUDIOROUTER_BRIDGE_CLOSE, &request)
    }

    pub fn close_with_mapping(
        &self,
        hello: &audiorouter_protocol::AudioBridgeHello,
        section_handle: u64,
        mapping_bytes: u32,
    ) -> Result<(), windows::core::Error> {
        let request =
            native_bridge_open_request(hello, section_handle, mapping_bytes).map_err(|_| {
                windows::core::Error::new(
                    windows::core::HRESULT(0x80070057u32 as i32),
                    "invalid hello or mapping",
                )
            })?;
        self.ioctl(IOCTL_AUDIOROUTER_BRIDGE_CLOSE, &request)
    }

    fn ioctl<T>(&self, code: u32, request: &T) -> Result<(), windows::core::Error> {
        use windows::Win32::System::IO::DeviceIoControl;
        let mut returned = 0;
        unsafe {
            DeviceIoControl(
                self.handle,
                code,
                Some(request as *const T as *const std::ffi::c_void),
                std::mem::size_of::<T>() as u32,
                None,
                0,
                Some(&mut returned),
                None,
            )?
        };
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for NativeBridgeControlClient {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(windows)]
fn native_bridge_open_request(
    hello: &audiorouter_protocol::AudioBridgeHello,
    section_handle: u64,
    mapping_bytes: u32,
) -> Result<NativeBridgeOpenRequest, ()> {
    hello.validate().map_err(|_| ())?;
    let encoded: Vec<u16> = hello.bus_id.encode_utf16().collect();
    if encoded.len() > 64 || encoded.len() * std::mem::size_of::<u16>() > 128 {
        return Err(());
    }
    let mut bus_id = [0; 64];
    bus_id[..encoded.len()].copy_from_slice(&encoded);
    if (section_handle == 0) != (mapping_bytes == 0) || (section_handle != 0 && mapping_bytes < 32)
    {
        return Err(());
    }
    Ok(NativeBridgeOpenRequest {
        protocol_major: hello.protocol_major,
        protocol_minor: hello.protocol_minor,
        bus_id_bytes: (encoded.len() * 2) as u16,
        channels: hello.channels,
        frames_per_quantum: hello.frames_per_quantum,
        sample_rate_hz: hello.sample_rate_hz,
        lease_ms: hello.lease_ms,
        generation: hello.generation,
        section_handle,
        mapping_bytes,
        reserved2: 0,
        bus_id,
        direction: match hello.direction {
            audiorouter_protocol::AudioBridgeDirection::RenderSource => 1,
            audiorouter_protocol::AudioBridgeDirection::CaptureSink => 2,
        },
    })
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
    pub channel_mask: u32,
    pub subformat_guid: String,
}

impl EndpointInfo {
    /// Whether this metadata describes interleaved IEEE float32 samples.
    /// Extensible formats are accepted only when their preserved subformat
    /// identity identifies IEEE float; a generic 32-bit container is not
    /// sufficient for callers that decode samples as `f32`.
    pub fn is_ieee_float32(&self) -> bool {
        self.bits_per_sample == 32
            && (self.format_tag == 3
                || (self.format_tag == 0xfffe
                    && self
                        .subformat_guid
                        .to_ascii_lowercase()
                        .contains("00000003")))
    }

    /// Return the interleaved byte stride implied by the endpoint's mix
    /// format. This is a checked metadata helper for packet-copy callers; it
    /// does not open or initialize the endpoint.
    pub fn bytes_per_frame(&self) -> Result<usize, AudioError> {
        if self.channels == 0 || self.bits_per_sample == 0 || self.bits_per_sample % 8 != 0 {
            return Err(AudioError::InvalidFrameSize);
        }
        (usize::from(self.channels))
            .checked_mul(usize::from(self.bits_per_sample / 8))
            .ok_or(AudioError::InvalidFrameSize)
    }
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

/// Result of resolving a previously persisted endpoint binding against a
/// fresh read-only snapshot. Missing and direction-changed bindings are
/// explicit so a reconnect path cannot silently substitute another device.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EndpointBindingResolution {
    Available(EndpointInfo),
    Missing {
        id: String,
        direction: EndpointDirection,
    },
    DirectionChanged {
        id: String,
        expected: EndpointDirection,
        actual: EndpointDirection,
    },
    FormatChanged {
        expected: EndpointInfo,
        actual: EndpointInfo,
    },
}

/// Resolve one exact opaque endpoint ID without opening or initializing it.
/// A caller may decide how to present `Missing` or `DirectionChanged`, but the
/// adapter deliberately does not choose a replacement endpoint.
pub fn resolve_endpoint_binding(
    snapshot: &[EndpointInfo],
    id: &str,
    direction: EndpointDirection,
) -> EndpointBindingResolution {
    match snapshot.iter().find(|endpoint| endpoint.id == id) {
        Some(endpoint) if endpoint.direction == direction => {
            EndpointBindingResolution::Available(endpoint.clone())
        }
        Some(endpoint) => EndpointBindingResolution::DirectionChanged {
            id: id.to_owned(),
            expected: direction,
            actual: endpoint.direction,
        },
        None => EndpointBindingResolution::Missing {
            id: id.to_owned(),
            direction,
        },
    }
}

/// Resolve a persisted binding while requiring its previously negotiated mix
/// format to remain unchanged. Format changes are surfaced for deliberate
/// renegotiation; this helper never opens a stream or silently resamples.
pub fn resolve_endpoint_binding_with_format(
    snapshot: &[EndpointInfo],
    expected: &EndpointInfo,
) -> EndpointBindingResolution {
    match resolve_endpoint_binding(snapshot, &expected.id, expected.direction) {
        EndpointBindingResolution::Available(actual)
            if actual.sample_rate_hz == expected.sample_rate_hz
                && actual.channels == expected.channels
                && actual.bits_per_sample == expected.bits_per_sample
                && actual.format_tag == expected.format_tag
                && actual.channel_mask == expected.channel_mask
                && actual.subformat_guid == expected.subformat_guid =>
        {
            EndpointBindingResolution::Available(actual)
        }
        EndpointBindingResolution::Available(actual) => EndpointBindingResolution::FormatChanged {
            expected: expected.clone(),
            actual,
        },
        other => other,
    }
}

/// Diff two metadata snapshots by opaque endpoint ID. This is a control-plane
/// polling helper; it never rebinding a missing endpoint or opens a stream.
pub fn diff_endpoint_snapshots(
    previous: &[EndpointInfo],
    current: &[EndpointInfo],
) -> Vec<EndpointChange> {
    // Windows does not promise a stable enumeration order. Sort private
    // clones so reconnect/event consumers receive reproducible change order
    // without mutating either caller-owned snapshot.
    let mut previous = previous.to_vec();
    let mut current = current.to_vec();
    previous.sort_by(|left, right| left.id.cmp(&right.id));
    current.sort_by(|left, right| left.id.cmp(&right.id));
    let mut changes = Vec::new();
    for before in &previous {
        match current.iter().find(|after| after.id == before.id) {
            Some(after) if after != before => changes.push(EndpointChange::Changed {
                before: before.clone(),
                after: after.clone(),
            }),
            Some(_) => {}
            None => changes.push(EndpointChange::Removed(before.clone())),
        }
    }
    for after in &current {
        if !previous.iter().any(|before| before.id == after.id) {
            changes.push(EndpointChange::Added(after.clone()));
        }
    }
    changes
}

/// Diff identity/state-only snapshots without requiring endpoint activation.
/// This seam is used for deterministic transition tests and for unavailable
/// devices whose format metadata cannot be queried.
pub fn diff_endpoint_state_snapshots(
    previous: &[EndpointStateInfo],
    current: &[EndpointStateInfo],
) -> Vec<EndpointStateChange> {
    let mut previous = previous.to_vec();
    let mut current = current.to_vec();
    previous.sort_by(|left, right| left.id.cmp(&right.id));
    current.sort_by(|left, right| left.id.cmp(&right.id));
    let mut changes = Vec::new();
    for before in &previous {
        match current.iter().find(|after| after.id == before.id) {
            Some(after) if after != before => changes.push(EndpointStateChange::Changed {
                before: before.clone(),
                after: after.clone(),
            }),
            Some(_) => {}
            None => changes.push(EndpointStateChange::Removed(before.clone())),
        }
    }
    for after in &current {
        if !previous.iter().any(|before| before.id == after.id) {
            changes.push(EndpointStateChange::Added(after.clone()));
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

/// Decode caller-owned interleaved IEEE float32 bytes into the engine's
/// caller-owned planar block. The native capture adapter must copy a WASAPI
/// packet into `source` and release the COM buffer before calling this helper.
/// No allocation, endpoint access, or format conversion occurs here.
pub fn decode_interleaved_float32(
    source: &[u8],
    channels: usize,
    destination: &mut audiorouter_engine::AudioBlock,
) -> Result<(), AudioError> {
    if channels == 0 || destination.channels() != channels {
        return Err(AudioError::InvalidFrameSize);
    }
    let expected = channels
        .checked_mul(destination.frames())
        .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>()))
        .ok_or(AudioError::InvalidFrameSize)?;
    if source.len() != expected {
        return Err(AudioError::BufferTooSmall {
            required: expected,
            available: source.len(),
        });
    }
    for frame in 0..destination.frames() {
        for channel in 0..channels {
            let offset = (frame * channels + channel) * std::mem::size_of::<f32>();
            let sample = f32::from_ne_bytes([
                source[offset],
                source[offset + 1],
                source[offset + 2],
                source[offset + 3],
            ]);
            destination.channel_mut(channel).expect("validated channel")[frame] =
                if sample.is_finite() { sample } else { 0.0 };
        }
    }
    Ok(())
}

/// Encode the engine's planar float32 block into caller-owned interleaved
/// IEEE float32 bytes for a WASAPI render packet. The destination must already
/// be sized for exactly one block; this helper never allocates or touches an
/// endpoint.
pub fn encode_interleaved_float32(
    source: &audiorouter_engine::AudioBlock,
    destination: &mut [u8],
) -> Result<(), AudioError> {
    let expected = source
        .channels()
        .checked_mul(source.frames())
        .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>()))
        .ok_or(AudioError::InvalidFrameSize)?;
    if destination.len() != expected {
        return Err(AudioError::BufferTooSmall {
            required: expected,
            available: destination.len(),
        });
    }
    for frame in 0..source.frames() {
        for channel in 0..source.channels() {
            let sample = source.channel(channel).expect("validated channel")[frame];
            let offset = (frame * source.channels() + channel) * std::mem::size_of::<f32>();
            destination[offset..offset + 4].copy_from_slice(&sample.to_ne_bytes());
        }
    }
    Ok(())
}

/// Maximum packet storage a native adapter may reserve for quantum
/// accumulation. A caller chooses a smaller bound when the endpoint permits
/// it; this ceiling prevents a device-reported period from causing an
/// unbounded preparation allocation.
pub const MAX_FLOAT32_ACCUMULATOR_FRAMES: usize = 4_096;
const RENDER_CARRY_QUANTA: usize = 64;

/// Fixed-capacity interleaved float32 packet accumulator for the engine's
/// fixed quantum. WASAPI packets may be shorter, longer, or split at an
/// unrelated period boundary. The accumulator accepts only complete frames,
/// returns the number of source bytes consumed, and yields complete quanta
/// through [`Self::pop_into`]. All storage is allocated by `new`; `push` and
/// `pop_into` never allocate, wait, or access Windows.
pub struct Float32PacketAccumulator {
    channels: usize,
    quantum_frames: usize,
    pending_frames: usize,
    bytes: Vec<u8>,
}

impl Float32PacketAccumulator {
    pub fn new(
        channels: usize,
        quantum_frames: usize,
        capacity_frames: usize,
    ) -> Result<Self, AudioError> {
        if channels == 0
            || quantum_frames == 0
            || capacity_frames < quantum_frames
            || capacity_frames > MAX_FLOAT32_ACCUMULATOR_FRAMES
        {
            return Err(AudioError::InvalidFrameSize);
        }
        let bytes = capacity_frames
            .checked_mul(channels)
            .and_then(|samples| samples.checked_mul(std::mem::size_of::<f32>()))
            .ok_or(AudioError::InvalidFrameSize)?;
        Ok(Self {
            channels,
            quantum_frames,
            pending_frames: 0,
            bytes: vec![0; bytes],
        })
    }

    pub fn pending_frames(&self) -> usize {
        self.pending_frames
    }

    /// Discard a partial packet during endpoint invalidation or stream
    /// replacement. Old samples must not be completed or replayed after a
    /// recovery boundary.
    pub fn reset(&mut self) {
        self.pending_frames = 0;
    }

    /// Copy as many complete interleaved frames as fit and return the number
    /// of source bytes consumed. The caller can submit the remainder on the
    /// next nonblocking iteration; no source memory is retained.
    pub fn push(&mut self, source: &[u8]) -> Result<usize, AudioError> {
        let bytes_per_frame = self.channels * std::mem::size_of::<f32>();
        if source.len() % bytes_per_frame != 0 {
            return Err(AudioError::InvalidFrameSize);
        }
        let capacity_frames = self.bytes.len() / bytes_per_frame;
        let writable_frames = capacity_frames.saturating_sub(self.pending_frames);
        let frames = writable_frames.min(source.len() / bytes_per_frame);
        let bytes = frames * bytes_per_frame;
        let start = self.pending_frames * bytes_per_frame;
        self.bytes[start..start + bytes].copy_from_slice(&source[..bytes]);
        self.pending_frames += frames;
        Ok(bytes)
    }

    /// Decode one complete quantum into a caller-owned engine block. Returns
    /// `false` when fewer than one quantum is pending; the pending bytes stay
    /// intact for the next packet.
    pub fn pop_into(
        &mut self,
        destination: &mut audiorouter_engine::AudioBlock,
    ) -> Result<bool, AudioError> {
        if destination.channels() != self.channels || destination.frames() != self.quantum_frames {
            return Err(AudioError::InvalidFrameSize);
        }
        if self.pending_frames < self.quantum_frames {
            return Ok(false);
        }
        let quantum_bytes = self.quantum_frames * self.channels * std::mem::size_of::<f32>();
        decode_interleaved_float32(&self.bytes[..quantum_bytes], self.channels, destination)?;
        let remaining_frames = self.pending_frames - self.quantum_frames;
        let remaining_bytes = remaining_frames * self.channels * std::mem::size_of::<f32>();
        self.bytes
            .copy_within(quantum_bytes..quantum_bytes + remaining_bytes, 0);
        self.pending_frames = remaining_frames;
        Ok(true)
    }
}

/// Result of one nonblocking endpoint pump. Counts are control/diagnostic
/// data; the pump itself does not log or allocate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WasapiSchedulerPump {
    pub packets: u32,
    pub captured_frames: u32,
    pub processed_quanta: u32,
    pub rendered_frames: u32,
    pub dropped_render_frames: u32,
}

impl WasapiSchedulerPump {
    fn accumulate(&mut self, other: Self) {
        self.packets = self.packets.saturating_add(other.packets);
        self.captured_frames = self.captured_frames.saturating_add(other.captured_frames);
        self.processed_quanta = self.processed_quanta.saturating_add(other.processed_quanta);
        self.rendered_frames = self.rendered_frames.saturating_add(other.rendered_frames);
        self.dropped_render_frames = self
            .dropped_render_frames
            .saturating_add(other.dropped_render_frames);
    }
}

/// Endpoint-owned composition boundary for the portable realtime scheduler.
/// The caller owns client activation, event waits, start/stop, graph
/// publication, and any tap. `pump` only performs bounded packet copying,
/// accumulation, graph steps, and render submission using storage allocated by
/// `new`; it never starts a client, waits, allocates, or changes endpoint
/// configuration.
pub struct WasapiSchedulerBridge {
    scheduler: audiorouter_engine::RealtimeScheduler,
    accumulator: Float32PacketAccumulator,
    capture_bytes: Vec<u8>,
    render_bytes: Vec<u8>,
    render_pending: Vec<u8>,
    bridge_samples: Vec<f32>,
    render_pending_bytes: usize,
    bytes_per_frame: usize,
    quantum_frames: usize,
    timeline_frame: u64,
}

impl WasapiSchedulerBridge {
    /// Construct a bridge only for two already-validated endpoint descriptors
    /// with the same IEEE float32 mix format. This check is intentionally
    /// metadata-only; callers still activate the exact bindings separately.
    pub fn new_for_endpoints(
        ring_capacity: usize,
        capture: &EndpointInfo,
        render: &EndpointInfo,
        quantum_frames: usize,
        max_packet_frames: usize,
    ) -> Result<Self, AudioError> {
        if capture.direction != EndpointDirection::Capture
            || render.direction != EndpointDirection::Render
            || !capture.is_ieee_float32()
            || !render.is_ieee_float32()
            || capture.sample_rate_hz != render.sample_rate_hz
            || capture.channels != render.channels
            || capture.bits_per_sample != render.bits_per_sample
            || capture.format_tag != render.format_tag
            || capture.channel_mask != render.channel_mask
            || !capture
                .subformat_guid
                .eq_ignore_ascii_case(&render.subformat_guid)
        {
            return Err(AudioError::InvalidFrameSize);
        }
        Self::new(
            ring_capacity,
            usize::from(capture.channels),
            quantum_frames,
            max_packet_frames,
        )
    }

    pub fn new(
        ring_capacity: usize,
        channels: usize,
        quantum_frames: usize,
        max_packet_frames: usize,
    ) -> Result<Self, AudioError> {
        if max_packet_frames == 0 || max_packet_frames > MAX_FLOAT32_ACCUMULATOR_FRAMES {
            return Err(AudioError::InvalidFrameSize);
        }
        let bytes_per_frame = channels
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or(AudioError::InvalidFrameSize)?;
        let capture_bytes = max_packet_frames
            .checked_mul(bytes_per_frame)
            .ok_or(AudioError::InvalidFrameSize)?;
        let render_bytes = quantum_frames
            .checked_mul(bytes_per_frame)
            .ok_or(AudioError::InvalidFrameSize)?;
        let render_pending = render_bytes
            .checked_mul(RENDER_CARRY_QUANTA)
            .ok_or(AudioError::InvalidFrameSize)?;
        let scheduler =
            audiorouter_engine::RealtimeScheduler::new(ring_capacity, channels, quantum_frames)
                .map_err(|_| AudioError::InvalidFrameSize)?;
        let accumulator = Float32PacketAccumulator::new(
            channels,
            quantum_frames,
            max_packet_frames.max(quantum_frames),
        )?;
        Ok(Self {
            scheduler,
            accumulator,
            capture_bytes: vec![0; capture_bytes],
            render_bytes: vec![0; render_bytes],
            render_pending: vec![0; render_pending],
            bridge_samples: vec![0.0; max_packet_frames * channels],
            render_pending_bytes: 0,
            bytes_per_frame,
            quantum_frames,
            timeline_frame: 0,
        })
    }

    pub fn scheduler(&self) -> &audiorouter_engine::RealtimeScheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut audiorouter_engine::RealtimeScheduler {
        &mut self.scheduler
    }

    pub fn timeline_frame(&self) -> u64 {
        self.timeline_frame
    }

    /// Clear all audio staged by the adapter at an endpoint lifecycle
    /// boundary. The prepared graph remains published, but partial capture,
    /// pending render, queued scheduler blocks, and the timeline are reset so
    /// a reopened client cannot replay audio from the invalidated stream.
    ///
    /// This is a control/recovery operation and must be called only after the
    /// endpoint clients have stopped; `pump` never invokes it.
    pub fn reset_stream(&mut self) -> Result<usize, AudioError> {
        self.accumulator.reset();
        self.render_pending_bytes = 0;
        self.timeline_frame = 0;
        let state_reset = self.scheduler.reset_processing_state();
        let recycled = self.scheduler.reset_io();
        if !state_reset {
            return Err(AudioError::ProcessingStateUnavailable);
        }
        Ok(recycled)
    }

    /// Copy and process at most the currently available capture packet. A
    /// render buffer with insufficient device capacity is explicitly dropped
    /// and counted rather than blocking the graph or retaining unbounded data.
    pub fn pump(
        &mut self,
        capture: &SharedCapture,
        render: &SharedRender,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        self.pump_internal(capture, render, None, None)
    }

    /// Pump capture/render data while forwarding each processed quantum to an
    /// allocation-free tap. The tap is borrowed only for this call and must
    /// obey the engine's realtime callback contract.
    pub fn pump_with_tap(
        &mut self,
        capture: &SharedCapture,
        render: &SharedRender,
        tap: &dyn audiorouter_engine::AudioTap,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        self.pump_internal(capture, render, Some(tap), None)
    }

    #[cfg(windows)]
    fn pump_from_native_render_source(
        &mut self,
        source: &impl NativeRenderSource,
        render: &dyn RenderSink,
        last_sequence: &mut u64,
    ) -> Result<WasapiSchedulerPump, NativeBridgeInputPumpError> {
        let mut result = WasapiSchedulerPump::default();
        self.drain_render_pending(render, &mut result)
            .map_err(NativeBridgeInputPumpError::Audio)?;
        let mut input = self
            .scheduler
            .acquire_input()
            .ok_or(NativeBridgeInputPumpError::Audio(
                AudioError::BufferTooSmall {
                    required: self.quantum_frames * self.bytes_per_frame,
                    available: 0,
                },
            ))?;
        let source_result = source.read_into_after(*last_sequence, &mut self.bridge_samples);
        match source_result {
            Ok(header) => {
                if usize::from(header.channels) != input.channels()
                    || usize::from(header.frames) != input.frames()
                {
                    input.clear();
                    let _ = self.scheduler.input().try_recycle(input);
                    return Err(NativeBridgeInputPumpError::Audio(
                        AudioError::InvalidFrameSize,
                    ));
                }
                if input
                    .copy_from_interleaved(
                        &self.bridge_samples[..input.channels() * input.frames()],
                    )
                    .is_err()
                {
                    let _ = self.scheduler.input().try_recycle(input);
                    return Err(NativeBridgeInputPumpError::Audio(
                        AudioError::InvalidFrameSize,
                    ));
                }
                *last_sequence = header.sequence;
            }
            Err(error) if native_bridge_input_error_is_silence(&error) => input.clear(),
            Err(error) => {
                input.clear();
                let _ = self.scheduler.input().try_recycle(input);
                return Err(NativeBridgeInputPumpError::Bridge(error));
            }
        }
        if let Err(input) = self.scheduler.submit_input(input) {
            let _ = self.scheduler.input().try_recycle(input);
            return Err(NativeBridgeInputPumpError::Audio(
                AudioError::BufferTooSmall {
                    required: self.quantum_frames * self.bytes_per_frame,
                    available: 0,
                },
            ));
        }
        result.packets = 1;
        let generation = self
            .scheduler
            .process_once()
            .map_err(|_| NativeBridgeInputPumpError::Audio(AudioError::InvalidFrameSize))?;
        self.timeline_frame = self
            .timeline_frame
            .saturating_add(self.quantum_frames as u64);
        result.processed_quanta = 1;
        if let Some(generation) = generation {
            if let Some(output) = self.scheduler.receive_output_for_generation(generation) {
                encode_interleaved_float32(&output, &mut self.render_bytes)
                    .map_err(NativeBridgeInputPumpError::Audio)?;
                let bytes = output.frames() * self.bytes_per_frame;
                if self.render_pending_bytes + bytes > self.render_pending.len() {
                    self.scheduler.output().try_recycle(output).ok();
                    return Err(NativeBridgeInputPumpError::Audio(
                        AudioError::BufferTooSmall {
                            required: self.render_pending_bytes + bytes,
                            available: self.render_pending.len(),
                        },
                    ));
                }
                self.render_pending[self.render_pending_bytes..self.render_pending_bytes + bytes]
                    .copy_from_slice(&self.render_bytes[..bytes]);
                self.render_pending_bytes += bytes;
                self.scheduler.output().try_recycle(output).ok();
                self.drain_render_pending(render, &mut result)
                    .map_err(NativeBridgeInputPumpError::Audio)?;
            }
        }
        Ok(result)
    }

    /// Pump with both a caller-owned processed-audio tap and one deadline
    /// observation for every complete quantum in this bounded pump.
    pub fn pump_with_tap_and_deadline(
        &mut self,
        capture: &SharedCapture,
        render: &SharedRender,
        tap: &dyn audiorouter_engine::AudioTap,
        deadline: std::time::Instant,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        self.pump_internal(
            capture,
            render,
            Some(tap),
            Some(DeadlineSchedule {
                timeline_frame: self.timeline_frame,
                first_deadline: deadline,
                quantum_duration: std::time::Duration::ZERO,
                quantum_frames: self.quantum_frames,
            }),
        )
    }

    /// Pump with a tap and a deadline schedule whose timestamps advance once
    /// per engine quantum. `first_deadline` describes the first quantum made
    /// ready by this call; subsequent quanta use `quantum_duration` offsets.
    /// The schedule is only an observation passed to the engine and never
    /// makes the audio path wait.
    pub fn pump_with_tap_and_quantum_deadline(
        &mut self,
        capture: &SharedCapture,
        render: &SharedRender,
        tap: &dyn audiorouter_engine::AudioTap,
        first_deadline: std::time::Instant,
        quantum_duration: std::time::Duration,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        let anchor = DeadlineSchedule {
            timeline_frame: self.timeline_frame,
            first_deadline,
            quantum_duration,
            quantum_frames: self.quantum_frames,
        };
        self.pump_internal(capture, render, Some(tap), Some(anchor))
    }

    fn pump_internal(
        &mut self,
        capture: &SharedCapture,
        render: &dyn RenderSink,
        tap: Option<&dyn audiorouter_engine::AudioTap>,
        deadline: Option<DeadlineSchedule>,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        let mut result = WasapiSchedulerPump::default();
        self.drain_render_pending(render, &mut result)?;
        let Some((packet, packet_bytes)) =
            capture.next_packet_into(&mut self.capture_bytes, self.bytes_per_frame)?
        else {
            return Ok(result);
        };
        result.packets = 1;
        result.captured_frames = packet.frames;
        let mut offset = 0;
        while offset < packet_bytes {
            let consumed = self
                .accumulator
                .push(&self.capture_bytes[offset..packet_bytes])?;
            offset += consumed;
            self.process_ready(render, &mut result, tap, deadline)?;
            if consumed == 0 {
                return Err(AudioError::BufferTooSmall {
                    required: self.bytes_per_frame,
                    available: 0,
                });
            }
        }
        self.process_ready(render, &mut result, tap, deadline)?;
        Ok(result)
    }

    fn process_ready(
        &mut self,
        render: &dyn RenderSink,
        result: &mut WasapiSchedulerPump,
        tap: Option<&dyn audiorouter_engine::AudioTap>,
        deadline: Option<DeadlineSchedule>,
    ) -> Result<(), AudioError> {
        while self.accumulator.pending_frames() >= self.quantum_frames {
            self.drain_render_pending(render, result)?;
            let Some(mut input) = self.scheduler.acquire_input() else {
                return Err(AudioError::BufferTooSmall {
                    required: self.quantum_frames * self.bytes_per_frame,
                    available: 0,
                });
            };
            if !self.accumulator.pop_into(&mut input)? {
                return Ok(());
            }
            self.scheduler
                .submit_input(input)
                .map_err(|_| AudioError::BufferTooSmall {
                    required: self.quantum_frames * self.bytes_per_frame,
                    available: 0,
                })?;
            let generation = match (tap, deadline) {
                (Some(tap), Some(schedule)) => self.scheduler.process_once_with_tap_and_deadline(
                    self.timeline_frame,
                    tap,
                    schedule.deadline_for(self.timeline_frame),
                ),
                (Some(tap), None) => self
                    .scheduler
                    .process_once_with_tap(self.timeline_frame, tap),
                (None, Some(schedule)) => self
                    .scheduler
                    .process_once_with_deadline(schedule.deadline_for(self.timeline_frame)),
                (None, None) => self.scheduler.process_once(),
            }
            .map_err(|_| AudioError::InvalidFrameSize)?;
            self.timeline_frame = self
                .timeline_frame
                .saturating_add(self.quantum_frames as u64);
            result.processed_quanta = result.processed_quanta.saturating_add(1);
            let Some(generation) = generation else {
                continue;
            };
            let Some(output) = self.scheduler.receive_output_for_generation(generation) else {
                continue;
            };
            encode_interleaved_float32(&output, &mut self.render_bytes)?;
            let output_bytes = output.frames() * self.bytes_per_frame;
            if self.render_pending_bytes + output_bytes > self.render_pending.len() {
                self.scheduler
                    .output()
                    .try_recycle(output)
                    .map_err(|_| AudioError::InvalidFrameSize)?;
                return Err(AudioError::BufferTooSmall {
                    required: self.render_pending_bytes + output_bytes,
                    available: self.render_pending.len(),
                });
            }
            self.render_pending
                [self.render_pending_bytes..self.render_pending_bytes + output_bytes]
                .copy_from_slice(&self.render_bytes[..output_bytes]);
            self.render_pending_bytes += output_bytes;
            self.scheduler
                .output()
                .try_recycle(output)
                .map_err(|_| AudioError::InvalidFrameSize)?;
            self.drain_render_pending(render, result)?;
        }
        Ok(())
    }

    fn drain_render_pending(
        &mut self,
        render: &dyn RenderSink,
        result: &mut WasapiSchedulerPump,
    ) -> Result<(), AudioError> {
        while self.render_pending_bytes > 0 {
            let submitted = render.submit_bytes(
                &self.render_pending[..self.render_pending_bytes],
                self.bytes_per_frame,
            )?;
            if submitted == 0 {
                break;
            }
            let submitted_bytes = (submitted as usize)
                .checked_mul(self.bytes_per_frame)
                .ok_or(AudioError::InvalidFrameSize)?;
            if submitted_bytes > self.render_pending_bytes {
                return Err(AudioError::InvalidFrameSize);
            }
            self.render_pending
                .copy_within(submitted_bytes..self.render_pending_bytes, 0);
            self.render_pending_bytes -= submitted_bytes;
            result.rendered_frames = result.rendered_frames.saturating_add(submitted);
        }
        Ok(())
    }
}

/// Owns the two explicitly selected endpoint clients and the bounded graph
/// bridge that connects them. Construction does not activate either client;
/// `start` is the only activation transition, and a render-start failure
/// stops the capture client before returning the error. `stop` always attempts
/// both endpoint stops and clears staged audio before reporting the first
/// failure. No endpoint is replaced automatically after invalidation.
pub struct WasapiEndpointWorker {
    capture: Option<SharedCapture>,
    render: Option<SharedRender>,
    bridge: WasapiSchedulerBridge,
    running: bool,
}

/// Maximum number of already-available packets drained by one worker wake.
/// Waiting for the next event remains outside the pump loop.
pub const MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE: u32 = 64;

trait EndpointLifecycle {
    fn start(&mut self) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
}

impl EndpointLifecycle for SharedCapture {
    fn start(&mut self) -> Result<(), AudioError> {
        SharedCapture::start(self)
    }
    fn stop(&mut self) -> Result<(), AudioError> {
        SharedCapture::stop(self)
    }
}

impl EndpointLifecycle for SharedRender {
    fn start(&mut self) -> Result<(), AudioError> {
        SharedRender::start(self)
    }
    fn stop(&mut self) -> Result<(), AudioError> {
        SharedRender::stop(self)
    }
}

fn start_endpoint_pair<C: EndpointLifecycle, R: EndpointLifecycle>(
    capture: &mut C,
    render: &mut R,
) -> Result<(), AudioError> {
    capture.start()?;
    if let Err(error) = render.start() {
        let _ = capture.stop();
        return Err(error);
    }
    Ok(())
}

fn stop_endpoint_pair<C: EndpointLifecycle, R: EndpointLifecycle>(
    capture: &mut C,
    render: &mut R,
) -> Result<(), AudioError> {
    let render_result = render.stop();
    let capture_result = capture.stop();
    render_result.and(capture_result)
}

fn stop_endpoint_pair_and_reset<C, R, F>(
    capture: &mut C,
    render: &mut R,
    reset: F,
) -> Result<(), AudioError>
where
    C: EndpointLifecycle,
    R: EndpointLifecycle,
    F: FnOnce() -> Result<(), AudioError>,
{
    let endpoint_result = stop_endpoint_pair(capture, render);
    let reset_result = reset();
    endpoint_result.and(reset_result)
}

impl WasapiEndpointWorker {
    /// Compose stopped, already-validated endpoint clients with their bridge.
    /// The clients must have matching shape validated by the bridge
    /// constructor; this method performs no activation or waiting.
    pub fn new(
        capture: SharedCapture,
        render: SharedRender,
        bridge: WasapiSchedulerBridge,
    ) -> Self {
        Self {
            capture: Some(capture),
            render: Some(render),
            bridge,
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn bridge(&self) -> &WasapiSchedulerBridge {
        &self.bridge
    }

    pub fn bridge_mut(&mut self) -> &mut WasapiSchedulerBridge {
        &mut self.bridge
    }

    /// Start capture first, then render. If render activation fails, capture
    /// is synchronously stopped before the failure is returned.
    pub fn start(&mut self) -> Result<(), AudioError> {
        if self.running {
            return Ok(());
        }
        let capture = self
            .capture
            .as_mut()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        let render = self
            .render
            .as_mut()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        start_endpoint_pair(capture, render)?;
        self.running = true;
        Ok(())
    }

    /// Stop both clients and reset all staged graph audio. Both stop calls are
    /// attempted even when the first one fails; the first endpoint/reset error
    /// is then returned to the control plane.
    pub fn stop(&mut self) -> Result<(), AudioError> {
        if !self.running {
            return self.bridge.reset_stream().map(|_| ());
        }
        self.running = false;
        let capture = self
            .capture
            .as_mut()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        let render = self
            .render
            .as_mut()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        stop_endpoint_pair_and_reset(capture, render, || self.bridge.reset_stream().map(|_| ()))
    }

    /// Stop and discard both old clients, refresh the monitor, and open only
    /// the exact persisted capture/render bindings. The worker remains
    /// stopped after a successful rebind; the caller must deliberately call
    /// `start`. Any open failure leaves the worker without clients so a stale
    /// endpoint cannot continue processing or be replaced implicitly.
    pub fn rebind_with_refreshed_bound_with_retry(
        &mut self,
        monitor: &mut EndpointMonitor,
        expected_capture: &EndpointInfo,
        expected_render: &EndpointInfo,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<(), AudioError> {
        let stop_result = self.stop();
        drop(self.capture.take());
        drop(self.render.take());
        stop_result?;

        let capture = SharedCapture::open_refreshed_bound_with_retry(
            monitor,
            expected_capture,
            buffer_duration_100ns,
            max_attempts,
            retry_delay_ms,
        )?;
        let render = match SharedRender::open_refreshed_bound_with_retry(
            monitor,
            expected_render,
            buffer_duration_100ns,
            max_attempts,
            retry_delay_ms,
        ) {
            Ok(render) => render,
            Err(error) => {
                drop(capture);
                return Err(error);
            }
        };
        self.capture = Some(capture);
        self.render = Some(render);
        Ok(())
    }

    /// Pump one bounded capture packet through the graph and into render.
    /// Endpoint activation and event waiting remain the caller's responsibility.
    pub fn pump(&mut self) -> Result<WasapiSchedulerPump, AudioError> {
        if !self.running {
            return Err(AudioError::ProcessingStateUnavailable);
        }
        let capture = self
            .capture
            .as_ref()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        let render = self
            .render
            .as_ref()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        self.bridge.pump(capture, render)
    }

    /// Drain at most the caller-selected bounded number of currently
    /// available packets. A packet-less pump ends the loop; this method never
    /// waits for a future packet or retries an invalidated endpoint.
    pub fn pump_available(&mut self, max_packets: u32) -> Result<WasapiSchedulerPump, AudioError> {
        if !self.running {
            return Err(AudioError::ProcessingStateUnavailable);
        }
        let mut total = WasapiSchedulerPump::default();
        let budget = max_packets.min(MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        for _ in 0..budget {
            let result = self.pump()?;
            total.accumulate(result);
            if result.packets == 0 {
                break;
            }
        }
        Ok(total)
    }

    /// Tap variant of [`Self::pump_available`] without imposing a deadline
    /// policy. The caller supplies scheduling policy when it owns the wakeup;
    /// this method retains the same bounded, non-waiting packet drain.
    pub fn pump_available_with_tap(
        &mut self,
        max_packets: u32,
        tap: &dyn audiorouter_engine::AudioTap,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        if !self.running {
            return Err(AudioError::ProcessingStateUnavailable);
        }
        let mut total = WasapiSchedulerPump::default();
        let budget = max_packets.min(MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        for _ in 0..budget {
            let capture = self
                .capture
                .as_ref()
                .ok_or(AudioError::ProcessingStateUnavailable)?;
            let render = self
                .render
                .as_ref()
                .ok_or(AudioError::ProcessingStateUnavailable)?;
            let result = self.bridge.pump_with_tap(capture, render, tap)?;
            total.accumulate(result);
            if result.packets == 0 {
                break;
            }
        }
        Ok(total)
    }

    /// Pump one bounded packet with the allocation-free graph tap and the
    /// rate-aware deadline observation used by the live adapter acceptance.
    pub fn pump_with_tap_and_quantum_deadline(
        &mut self,
        tap: &dyn audiorouter_engine::AudioTap,
        first_deadline: std::time::Instant,
        quantum_duration: std::time::Duration,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        if !self.running {
            return Err(AudioError::ProcessingStateUnavailable);
        }
        let capture = self
            .capture
            .as_ref()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        let render = self
            .render
            .as_ref()
            .ok_or(AudioError::ProcessingStateUnavailable)?;
        self.bridge.pump_with_tap_and_quantum_deadline(
            capture,
            render,
            tap,
            first_deadline,
            quantum_duration,
        )
    }

    /// Tap/deadline variant of [`Self::pump_available`], with the same hard
    /// packet budget and no wait or implicit recovery.
    pub fn pump_available_with_tap_and_quantum_deadline(
        &mut self,
        max_packets: u32,
        tap: &dyn audiorouter_engine::AudioTap,
        first_deadline: std::time::Instant,
        quantum_duration: std::time::Duration,
    ) -> Result<WasapiSchedulerPump, AudioError> {
        if !self.running {
            return Err(AudioError::ProcessingStateUnavailable);
        }
        let mut total = WasapiSchedulerPump::default();
        let budget = max_packets.min(MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE);
        for _ in 0..budget {
            let result =
                self.pump_with_tap_and_quantum_deadline(tap, first_deadline, quantum_duration)?;
            total.accumulate(result);
            if result.packets == 0 {
                break;
            }
        }
        Ok(total)
    }
}

impl Drop for WasapiEndpointWorker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Clone, Copy, Debug)]
struct DeadlineSchedule {
    timeline_frame: u64,
    first_deadline: std::time::Instant,
    quantum_duration: std::time::Duration,
    quantum_frames: usize,
}

impl DeadlineSchedule {
    fn deadline_for(self, timeline_frame: u64) -> std::time::Instant {
        let quantum_index =
            timeline_frame.saturating_sub(self.timeline_frame) / self.quantum_frames as u64;
        let quantum_index = u32::try_from(quantum_index).unwrap_or(u32::MAX);
        self.first_deadline
            .checked_add(self.quantum_duration.saturating_mul(quantum_index))
            .unwrap_or(self.first_deadline)
    }
}

/// Maximum packet period accepted by the process-loopback adapter before a
/// packet can be split into the fixed 128-frame engine quantum. Larger device
/// periods are rejected rather than creating an unbounded staging request.
pub const MAX_PROCESS_LOOPBACK_PACKET_FRAMES: u32 = 4_096;

fn validate_process_loopback_packet_frames(frames: u32) -> Result<(), AudioError> {
    (frames > 0 && frames <= MAX_PROCESS_LOOPBACK_PACKET_FRAMES)
        .then_some(())
        .ok_or(AudioError::InvalidFrameSize)
}

/// Bounded process-loopback delivery counters. Values are snapshots of the
/// adapter lifetime and are intended for control-plane diagnostics, not audio
/// callback logging or timing claims.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProcessLoopbackTelemetry {
    pub wait_calls: u64,
    pub wait_timeouts: u64,
    pub packets: u64,
    pub frames: u64,
    pub minimum_packet_frames: u32,
    pub maximum_packet_frames: u32,
    pub silent_packets: u64,
    pub rejected_packets: u64,
}

#[derive(Debug, Default)]
struct ProcessLoopbackTelemetryCounters {
    wait_calls: AtomicU64,
    wait_timeouts: AtomicU64,
    packets: AtomicU64,
    frames: AtomicU64,
    minimum_packet_frames: AtomicU64,
    maximum_packet_frames: AtomicU64,
    silent_packets: AtomicU64,
    rejected_packets: AtomicU64,
}

fn saturating_increment(counter: &AtomicU64) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_add(1))
    });
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationInfo {
    pub process_id: u32,
    pub executable: String,
    /// Verified full executable path when Windows permits limited process
    /// inspection. This distinguishes same-named binaries in different
    /// locations without exposing command lines or arbitrary process data.
    pub executable_path: Option<String>,
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
    pub render_session_count: u32,
    pub display_names: Vec<String>,
}

fn sort_application_inventory(applications: &mut [ApplicationInfo]) {
    applications.sort_by(|left, right| {
        left.executable
            .to_ascii_lowercase()
            .cmp(&right.executable.to_ascii_lowercase())
            .then_with(|| left.process_id.cmp(&right.process_id))
    });
}

fn sort_application_audio_inventory(inventory: &mut [ApplicationAudioInfo]) {
    inventory.sort_by_key(|item| item.process_id);
    for item in inventory {
        item.display_names.sort_unstable();
        item.display_names.dedup();
    }
}

fn retain_audio_display_name(display_names: &mut Vec<String>, name: String) {
    if name.is_empty()
        || name.len() > MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES
        || display_names.len() >= MAX_APPLICATION_AUDIO_DISPLAY_NAMES
        || display_names.iter().any(|existing| existing == &name)
    {
        return;
    }
    display_names.push(name);
}

#[derive(Debug)]
pub enum AudioError {
    Windows(windows::core::Error),
    WindowsOperation {
        operation: &'static str,
        error: windows::core::Error,
    },
    InvalidUtf16,
    BufferTooSmall {
        required: usize,
        available: usize,
    },
    InvalidFrameSize,
    ProcessingStateUnavailable,
    ApplicationNotFound {
        process_id: u32,
    },
    ApplicationIdentityChanged {
        process_id: u32,
    },
    ApplicationIdentityUnavailable {
        process_id: u32,
    },
    ApplicationRestartNotFound {
        executable: String,
    },
    ApplicationRestartAmbiguous {
        executable: String,
    },
    ApplicationRestartIdentityUnavailable {
        executable: String,
    },
    EndpointBinding {
        resolution: Box<EndpointBindingResolution>,
    },
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

impl AudioFailureKind {
    /// Stable machine-readable category for control-plane and CLI consumers.
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalidArgument",
            Self::AccessDenied => "accessDenied",
            Self::DeviceInUse => "deviceInUse",
            Self::ExclusiveModeOnly => "exclusiveModeOnly",
            Self::DeviceInvalidated => "deviceInvalidated",
            Self::UnsupportedFormat => "unsupportedFormat",
            Self::ServiceUnavailable => "serviceUnavailable",
            Self::BufferConstraint => "bufferConstraint",
            Self::Other => "other",
        }
    }

    /// Whether retrying after a transient system change is meaningful.
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::DeviceInUse | Self::DeviceInvalidated | Self::ServiceUnavailable
        )
    }

    /// Stable, user-facing guidance for control-plane error responses.
    pub fn remediation(self) -> &'static str {
        match self {
            Self::InvalidArgument => "correct the endpoint format or stream request",
            Self::AccessDenied => "grant the required Windows audio permission",
            Self::DeviceInUse => {
                "identify the owning stream, select another endpoint, or close it and retry"
            }
            Self::ExclusiveModeOnly => "use a compatible shared-mode endpoint or exclusive stream",
            Self::DeviceInvalidated => "refresh endpoint inventory and retry the verified endpoint",
            Self::UnsupportedFormat => "select an endpoint with a supported shared format",
            Self::ServiceUnavailable => "wait for the Windows audio service and retry",
            Self::BufferConstraint => {
                "reduce the requested packet size or provide more buffer space"
            }
            Self::Other => "inspect the HRESULT and correct the reported Windows audio failure",
        }
    }
}

impl AudioError {
    /// Return the structured endpoint-binding decision, when this error was
    /// produced by a bound stream-open operation. Callers can use this to
    /// distinguish disappearance, direction changes, and format renegotiation
    /// without parsing diagnostic text.
    pub fn binding_resolution(&self) -> Option<&EndpointBindingResolution> {
        match self {
            Self::EndpointBinding { resolution } => Some(resolution),
            _ => None,
        }
    }

    /// Return the stable unsigned HRESULT represented by this error. Local
    /// validation failures use the corresponding Win32 HRESULT so callers can
    /// emit one machine-readable diagnostic without parsing display text.
    pub fn hresult(&self) -> u32 {
        match self {
            Self::Windows(error) => error.code().0 as u32,
            Self::WindowsOperation { error, .. } => error.code().0 as u32,
            Self::InvalidUtf16
            | Self::InvalidFrameSize
            | Self::ApplicationNotFound { .. }
            | Self::ApplicationIdentityChanged { .. }
            | Self::ApplicationRestartNotFound { .. }
            | Self::ApplicationRestartAmbiguous { .. }
            | Self::BufferTooSmall { .. }
            | Self::EndpointBinding { .. } => 0x80070057,
            Self::ProcessingStateUnavailable => 0x80004005,
            Self::ApplicationIdentityUnavailable { .. }
            | Self::ApplicationRestartIdentityUnavailable { .. } => 0x80070005,
        }
    }

    /// Classify errors for stable control-plane behavior while retaining the
    /// original HRESULT for diagnostics.
    pub fn kind(&self) -> AudioFailureKind {
        if matches!(self, Self::BufferTooSmall { .. }) {
            return AudioFailureKind::BufferConstraint;
        }
        let code = self.hresult();
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

    pub fn is_retryable(&self) -> bool {
        self.kind().is_retryable()
    }

    pub fn remediation(&self) -> &'static str {
        self.kind().remediation()
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows(error) => write!(
                formatter,
                "Windows audio error 0x{:08X}: {error}",
                error.code().0 as u32
            ),
            Self::WindowsOperation { operation, error } => write!(
                formatter,
                "{operation} failed with Windows audio HRESULT 0x{:08X}: {error}",
                error.code().0 as u32
            ),
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
            Self::ProcessingStateUnavailable => {
                formatter.write_str("audio processor state could not be reset safely")
            }
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
            Self::EndpointBinding { resolution } => {
                write!(
                    formatter,
                    "audio endpoint binding is not available: {resolution:?}"
                )
            }
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

fn should_retry_capture_initialization(error: &windows::core::Error) -> bool {
    error.code() == windows::core::HRESULT(0x80070057u32 as i32)
}

fn capture_initialize_operation(event_driven: bool) -> &'static str {
    if event_driven {
        "IAudioClient::Initialize(capture,event-callback)"
    } else {
        "IAudioClient::Initialize(capture,polling)"
    }
}

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
    event: Option<EventHandle>,
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

/// The process tree selection supported by Windows process-loopback capture.
/// The selection is explicit because Windows supports one target tree, not an
/// arbitrary exclusion list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessLoopbackMode {
    IncludeTargetTree,
    ExcludeTargetTree,
}

/// A bounded process-loopback capture client. Windows activates this client
/// asynchronously through the process-loopback virtual device; the caller
/// still owns packet copying and must not retain WASAPI's borrowed buffer.
pub struct ProcessLoopbackCapture {
    client: windows::Win32::Media::Audio::IAudioClient,
    capture: windows::Win32::Media::Audio::IAudioCaptureClient,
    bytes_per_frame: usize,
    started: bool,
    event: EventHandle,
    telemetry: ProcessLoopbackTelemetryCounters,
    _com: ComApartment,
}

struct ProcessLoopbackCompletion {
    // A COM interface cannot be Send/Sync. The callback transfers its one
    // owned reference as an integer address; the activating thread rebuilds
    // the IUnknown immediately after the wait and owns the release thereafter.
    result: Option<Result<usize, windows::core::HRESULT>>,
}

#[windows::core::implement(windows::Win32::Media::Audio::IActivateAudioInterfaceCompletionHandler)]
struct ProcessLoopbackCompletionHandler {
    completion: Arc<(
        std::sync::Mutex<ProcessLoopbackCompletion>,
        std::sync::Condvar,
    )>,
}

impl windows::Win32::Media::Audio::IActivateAudioInterfaceCompletionHandler_Impl
    for ProcessLoopbackCompletionHandler_Impl
{
    fn ActivateCompleted(
        &self,
        operation: windows::core::Ref<
            windows::Win32::Media::Audio::IActivateAudioInterfaceAsyncOperation,
        >,
    ) -> windows::core::Result<()> {
        let mut activation_result = windows::core::HRESULT(0);
        let mut activated_interface = None;
        let result = unsafe {
            match operation.ok().and_then(|operation| {
                operation.GetActivateResult(&mut activation_result, &mut activated_interface)
            }) {
                Ok(()) if activation_result.is_ok() => match activated_interface {
                    Some(interface) => Ok(interface.into_raw() as usize),
                    None => Err(windows::core::HRESULT(0x80004002u32 as i32)),
                },
                Ok(()) => Err(activation_result),
                Err(error) => Err(error.code()),
            }
        };
        let (lock, wake) = &*self.completion;
        if let Ok(mut state) = lock.lock() {
            state.result = Some(result);
            wake.notify_one();
        }
        Ok(())
    }
}

impl ProcessLoopbackCapture {
    /// Activate one process tree using the supported asynchronous Windows
    /// process-loopback path. The process ID is only used as the activation
    /// target; callers that persist bindings must separately verify process
    /// creation identity before calling this method.
    pub fn open(process_id: u32, mode: ProcessLoopbackMode) -> Result<Self, AudioError> {
        use windows::Win32::Media::Audio::{
            ActivateAudioInterfaceAsync, IAudioClient, AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            AUDCLNT_STREAMFLAGS_LOOPBACK, AUDIOCLIENT_ACTIVATION_PARAMS,
            AUDIOCLIENT_ACTIVATION_PARAMS_0, AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS, PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE,
            PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
            VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, WAVEFORMATEX, WAVE_FORMAT_PCM,
        };
        use windows::Win32::System::Com::StructuredStorage::{
            PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
        };
        use windows::Win32::System::Com::{CoTaskMemAlloc, BLOB};
        use windows::Win32::System::Variant::VT_BLOB;

        if process_id == 0 {
            return Err(AudioError::ApplicationNotFound { process_id });
        }
        let com = ComApartment::initialize()?;
        let process_params = AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
            TargetProcessId: process_id,
            ProcessLoopbackMode: match mode {
                ProcessLoopbackMode::IncludeTargetTree => {
                    PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE
                }
                ProcessLoopbackMode::ExcludeTargetTree => {
                    PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE
                }
            },
        };
        let activation_params = AUDIOCLIENT_ACTIVATION_PARAMS {
            ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
                ProcessLoopbackParams: process_params,
            },
        };
        let blob_size = std::mem::size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>();
        let blob_data = unsafe { CoTaskMemAlloc(blob_size) };
        if blob_data.is_null() {
            return Err(AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x8007000Eu32 as i32),
                "process-loopback activation allocation failed",
            )));
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                std::ptr::addr_of!(activation_params).cast::<u8>(),
                blob_data.cast::<u8>(),
                blob_size,
            );
        }
        let property = PROPVARIANT {
            Anonymous: PROPVARIANT_0 {
                Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                    vt: VT_BLOB,
                    wReserved1: 0,
                    wReserved2: 0,
                    wReserved3: 0,
                    Anonymous: PROPVARIANT_0_0_0 {
                        blob: BLOB {
                            cbSize: blob_size as u32,
                            pBlobData: blob_data.cast::<u8>(),
                        },
                    },
                }),
            },
        };
        let completion = Arc::new((
            std::sync::Mutex::new(ProcessLoopbackCompletion { result: None }),
            std::sync::Condvar::new(),
        ));
        let handler: windows::Win32::Media::Audio::IActivateAudioInterfaceCompletionHandler =
            ProcessLoopbackCompletionHandler {
                completion: Arc::clone(&completion),
            }
            .into();
        let operation = unsafe {
            ActivateAudioInterfaceAsync(
                VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
                &IAudioClient::IID,
                Some(std::ptr::addr_of!(property)),
                &handler,
            )
        }
        .map_err(|error| AudioError::WindowsOperation {
            operation: "ActivateAudioInterfaceAsync(process-loopback)",
            error,
        })?;
        let (lock, wake) = &*completion;
        let state = lock.lock().map_err(|_| {
            AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80004005u32 as i32),
                "process-loopback activation state was poisoned",
            ))
        })?;
        let timeout = std::time::Duration::from_secs(5);
        let (mut state, timed_out) = wake
            .wait_timeout_while(state, timeout, |state| state.result.is_none())
            .map_err(|_| {
                AudioError::Windows(windows::core::Error::new(
                    windows::core::HRESULT(0x80004005u32 as i32),
                    "process-loopback activation wait was poisoned",
                ))
            })?;
        if timed_out.timed_out() && state.result.is_none() {
            // Windows may still invoke the callback and read the PROPVARIANT.
            // Leak the operation, handler, completion, and blob together on
            // this exceptional timeout rather than creating a use-after-free.
            std::mem::forget(operation);
            std::mem::forget(handler);
            std::mem::forget(property);
            return Err(AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x800705B4u32 as i32),
                "process-loopback activation timed out",
            )));
        }
        let client_address = state
            .result
            .take()
            .ok_or_else(|| {
                windows::core::Error::new(
                    windows::core::HRESULT(0x80004005u32 as i32),
                    "process-loopback activation returned no client",
                )
            })
            .and_then(|result| result.map_err(windows::core::Error::from))
            .map_err(|error| AudioError::WindowsOperation {
                operation: "ActivateAudioInterfaceAsync(process-loopback)",
                error,
            })?;
        drop(state);
        let client_unknown =
            unsafe { windows::core::IUnknown::from_raw(client_address as *mut std::ffi::c_void) };
        let client: IAudioClient =
            client_unknown
                .cast()
                .map_err(|error| AudioError::WindowsOperation {
                    operation: "ActivateAudioInterfaceAsync(process-loopback)/QueryInterface",
                    error,
                })?;
        // The process-loopback virtual device does not expose a reliable
        // endpoint mix format. Microsoft’s activation sample uses this
        // caller-owned PCM shape and lets shared mode convert it.
        let format = WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_PCM as u16,
            nChannels: 2,
            nSamplesPerSec: 44_100,
            nAvgBytesPerSec: 176_400,
            nBlockAlign: 4,
            wBitsPerSample: 16,
            cbSize: 0,
        };
        let bytes_per_frame = usize::from(format.nBlockAlign);
        let event = EventHandle(unsafe {
            windows::Win32::System::Threading::CreateEventW(None, false, false, None)?
        });
        let initialized = unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK
                    | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                    | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                0,
                0,
                &format,
                None,
            )
        };
        initialized.map_err(|error| AudioError::WindowsOperation {
            operation: "IAudioClient::Initialize(process-loopback)",
            error,
        })?;
        unsafe { client.SetEventHandle(event.0)? };
        let capture: windows::Win32::Media::Audio::IAudioCaptureClient =
            unsafe { client.GetService()? };
        drop(operation);
        Ok(Self {
            client,
            capture,
            bytes_per_frame,
            started: false,
            event,
            telemetry: ProcessLoopbackTelemetryCounters::default(),
            _com: com,
        })
    }

    pub fn start(&mut self) -> Result<(), AudioError> {
        unsafe { self.client.Start()? };
        self.started = true;
        Ok(())
    }

    /// Bytes in one interleaved frame of the activated process-loopback mix
    /// format. The value is fixed by the supported caller-owned PCM request
    /// and is safe to use when sizing caller-owned packet storage.
    pub fn bytes_per_frame(&self) -> usize {
        self.bytes_per_frame
    }

    /// Wait for the process-loopback event callback to signal available data.
    /// The timeout is bounded by the caller; a timeout returns `false` and
    /// does not inspect or alter endpoint state.
    pub fn wait_for_data(&self, timeout_ms: u32) -> Result<bool, AudioError> {
        saturating_increment(&self.telemetry.wait_calls);
        let result = unsafe {
            windows::Win32::System::Threading::WaitForSingleObject(self.event.0, timeout_ms)
        };
        if result == windows::Win32::Foundation::WAIT_OBJECT_0 {
            Ok(true)
        } else if result == windows::Win32::Foundation::WAIT_TIMEOUT {
            saturating_increment(&self.telemetry.wait_timeouts);
            Ok(false)
        } else {
            Err(AudioError::Windows(windows::core::Error::from_thread()))
        }
    }

    /// Read a point-in-time delivery snapshot without waiting or allocating.
    pub fn telemetry(&self) -> ProcessLoopbackTelemetry {
        let packets = self.telemetry.packets.load(Ordering::Relaxed);
        ProcessLoopbackTelemetry {
            wait_calls: self.telemetry.wait_calls.load(Ordering::Relaxed),
            wait_timeouts: self.telemetry.wait_timeouts.load(Ordering::Relaxed),
            packets,
            frames: self.telemetry.frames.load(Ordering::Relaxed),
            minimum_packet_frames: if packets == 0 {
                0
            } else {
                self.telemetry.minimum_packet_frames.load(Ordering::Relaxed) as u32
            },
            maximum_packet_frames: self.telemetry.maximum_packet_frames.load(Ordering::Relaxed)
                as u32,
            silent_packets: self.telemetry.silent_packets.load(Ordering::Relaxed),
            rejected_packets: self.telemetry.rejected_packets.load(Ordering::Relaxed),
        }
    }

    /// Copy one available packet into a caller-owned byte buffer. No borrowed
    /// WASAPI pointer escapes this method, and silent packets are represented
    /// by zero-filled caller storage.
    pub fn read_packet(&self, destination: &mut [u8]) -> Result<Option<CapturePacket>, AudioError> {
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
        }
        if validate_process_loopback_packet_frames(packet_frames).is_err() {
            unsafe { self.capture.ReleaseBuffer(packet_frames)? };
            saturating_increment(&self.telemetry.rejected_packets);
            return Err(AudioError::InvalidFrameSize);
        }
        let required = (packet_frames as usize)
            .checked_mul(self.bytes_per_frame)
            .ok_or(AudioError::InvalidFrameSize)?;
        if required > destination.len() {
            unsafe { self.capture.ReleaseBuffer(packet_frames)? };
            return Err(AudioError::BufferTooSmall {
                required,
                available: destination.len(),
            });
        }
        if flags & windows::Win32::Media::Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
            destination[..required].fill(0);
        } else {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.cast::<u8>(),
                    destination.as_mut_ptr(),
                    required,
                );
            }
        }
        unsafe { self.capture.ReleaseBuffer(packet_frames)? };
        saturating_increment(&self.telemetry.packets);
        let _ = self
            .telemetry
            .frames
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_add(u64::from(packet_frames)))
            });
        let _ = self.telemetry.minimum_packet_frames.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |value| {
                Some(if value == 0 {
                    u64::from(packet_frames)
                } else {
                    value.min(u64::from(packet_frames))
                })
            },
        );
        let _ = self.telemetry.maximum_packet_frames.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |value| Some(value.max(u64::from(packet_frames))),
        );
        if flags & windows::Win32::Media::Audio::AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
            saturating_increment(&self.telemetry.silent_packets);
        }
        Ok(Some(CapturePacket {
            frames: packet_frames,
            flags,
            device_position,
            qpc_position,
        }))
    }

    pub fn stop(&mut self) -> Result<(), AudioError> {
        // Reset is a required cleanup attempt even when Stop reports a
        // device/service failure. Clear local state first so Drop and a
        // caller's retry cannot issue a second Stop against a lost client.
        let stop_result = if self.started {
            self.started = false;
            unsafe { self.client.Stop() }.map_err(AudioError::from)
        } else {
            Ok(())
        };
        let reset_result = unsafe { self.client.Reset() }.map_err(AudioError::from);
        stop_result.and(reset_result)
    }
}

impl Drop for ProcessLoopbackCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl SharedCapture {
    /// Stop, release, refresh, and reopen the exact capture binding with a
    /// bounded retry policy for transient device/service failures. The old
    /// client is released before any retry, and no substitute endpoint is
    /// ever selected.
    pub fn replace_with_refreshed_bound_with_retry(
        mut self,
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<Self, AudioError> {
        let stop_result = self.stop();
        drop(self);
        stop_result?;
        Self::open_refreshed_bound_with_retry(
            monitor,
            expected,
            buffer_duration_100ns,
            max_attempts,
            retry_delay_ms,
        )
    }

    /// Refresh and reopen the exact capture binding with a bounded retry
    /// policy for transient device/service failures. This helper is intended
    /// for the control/recovery thread; it never sleeps on an audio callback
    /// and never selects a substitute endpoint.
    pub fn open_refreshed_bound_with_retry(
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<Self, AudioError> {
        retry_transient_audio_operation(max_attempts, retry_delay_ms, || {
            Self::open_refreshed_bound(monitor, expected, buffer_duration_100ns)
        })
    }

    /// Stop and release this capture client, refresh endpoint metadata, and
    /// open the same verified binding as a new client. The old client is
    /// always dropped before activation; missing/direction-changed/format-
    /// changed bindings fail closed without selecting a substitute.
    pub fn replace_with_refreshed_bound(
        mut self,
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
    ) -> Result<Self, AudioError> {
        let stop_result = self.stop();
        drop(self);
        stop_result?;
        Self::open_refreshed_bound(monitor, expected, buffer_duration_100ns)
    }

    /// Refresh endpoint metadata and then open only the exact persisted
    /// capture binding. This is the preferred recovery entry point because a
    /// coalesced notification cannot leave validation against an old snapshot.
    pub fn open_refreshed_bound(
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
    ) -> Result<Self, AudioError> {
        monitor.refresh_changes()?;
        Self::open_bound(monitor, expected, buffer_duration_100ns)
    }

    /// Open an endpoint only when the monitor still reports the exact
    /// persisted ID, direction, and mix format. A stale binding fails before
    /// COM activation; the caller must deliberately renegotiate or select a
    /// replacement. Endpoint changes between this check and activation remain
    /// visible as the underlying WASAPI error.
    pub fn open_bound(
        monitor: &EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
    ) -> Result<Self, AudioError> {
        let endpoint_id = bound_endpoint_id(monitor, expected, EndpointDirection::Capture)?;
        Self::open(&endpoint_id, buffer_duration_100ns)
    }

    /// Open an exact active capture endpoint using its opaque endpoint ID.
    /// The stream is initialized but remains stopped until `start` is called.
    /// The duration argument is retained for API compatibility; event-driven
    /// shared-mode WASAPI requires `Initialize` to receive zero here.
    pub fn open(endpoint_id: &str, _buffer_duration_100ns: i64) -> Result<Self, AudioError> {
        match Self::open_internal(endpoint_id, true, 0) {
            Err(AudioError::WindowsOperation { error, .. })
                if should_retry_capture_initialization(&error) =>
            {
                // The native M00 probe qualifies this exact fallback request.
                // Retry only E_INVALIDARG: AUDCLNT_E_DEVICE_IN_USE,
                // permission failures, and endpoint disappearance must remain
                // visible to the caller instead of being relabeled as a mode
                // compatibility issue.
                Self::open_internal(endpoint_id, false, 1_000_000)
            }
            result => result,
        }
    }

    /// Open a shared capture endpoint with timer/polling delivery.
    ///
    /// Some endpoint drivers accept the exact mix format for ordinary shared
    /// initialization but reject the additional event-callback request with
    /// `E_INVALIDARG`. The native M00 reference path qualifies this mode with
    /// a bounded buffer duration. The normal `open` path retries this mode only
    /// for the exact `E_INVALIDARG` event-callback incompatibility.
    pub fn open_polling(endpoint_id: &str, buffer_duration_100ns: i64) -> Result<Self, AudioError> {
        let duration = if buffer_duration_100ns > 0 {
            buffer_duration_100ns
        } else {
            1_000_000
        };
        Self::open_internal(endpoint_id, false, duration)
    }

    fn open_internal(
        endpoint_id: &str,
        event_driven: bool,
        buffer_duration_100ns: i64,
    ) -> Result<Self, AudioError> {
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
        let event = if event_driven {
            Some(EventHandle(unsafe {
                windows::Win32::System::Threading::CreateEventW(None, false, false, None)?
            }))
        } else {
            None
        };
        // Create the event before requesting the COM-allocated format so an
        // event-creation failure cannot leak the format buffer.
        let format = unsafe { client.GetMixFormat()? };
        let stream_flags = if event_driven {
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_NOPERSIST
        } else {
            AUDCLNT_STREAMFLAGS_NOPERSIST
        };
        let initialized = unsafe {
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                // GetMixFormat is the endpoint's exact shared-mode format;
                // conversion is neither needed nor desirable here. Keeping
                // the request exact avoids format/flag combinations that
                // some drivers reject with E_INVALIDARG.
                stream_flags,
                buffer_duration_100ns,
                0,
                format,
                None,
            )
        };
        unsafe { windows::Win32::System::Com::CoTaskMemFree(Some(format.cast())) };
        initialized.map_err(|error| AudioError::WindowsOperation {
            operation: capture_initialize_operation(event_driven),
            error,
        })?;
        if let Some(event) = &event {
            unsafe { client.SetEventHandle(event.0)? };
        }
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
            let packet_bytes = match (packet_frames as usize).checked_mul(bytes_per_frame) {
                Some(bytes) => bytes,
                None => {
                    // GetBuffer has transferred ownership of the packet
                    // until ReleaseBuffer, including when post-acquisition
                    // validation fails.
                    if let Err(error) = self.capture.ReleaseBuffer(packet_frames) {
                        return Err(AudioError::from(error));
                    }
                    return Err(AudioError::InvalidFrameSize);
                }
            };
            if packet_bytes > destination.len() {
                if let Err(error) = self.capture.ReleaseBuffer(packet_frames) {
                    return Err(AudioError::from(error));
                }
                return Err(AudioError::BufferTooSmall {
                    required: packet_bytes,
                    available: destination.len(),
                });
            }
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

    /// Wait for available data. Event-driven clients wait on their callback
    /// handle; polling clients use bounded packet-size polling. Packet reads
    /// remain explicit and bounded; a timeout is reported as `false`.
    pub fn wait_for_data(&self, timeout_ms: u32) -> Result<bool, AudioError> {
        let Some(event) = &self.event else {
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(u64::from(timeout_ms)))
                .unwrap_or_else(std::time::Instant::now);
            loop {
                if unsafe { self.capture.GetNextPacketSize()? } != 0 {
                    return Ok(true);
                }
                if std::time::Instant::now() >= deadline {
                    return Ok(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        };
        let result =
            unsafe { windows::Win32::System::Threading::WaitForSingleObject(event.0, timeout_ms) };
        if result == windows::Win32::Foundation::WAIT_OBJECT_0 {
            Ok(true)
        } else if result == windows::Win32::Foundation::WAIT_TIMEOUT {
            Ok(false)
        } else {
            Err(AudioError::Windows(windows::core::Error::from_thread()))
        }
    }

    pub fn stop(&mut self) -> Result<(), AudioError> {
        // Reset is attempted after Stop regardless of Stop's result so a
        // partial device failure cannot leave the client's queued state
        // unreleased. The local flag is cleared before the COM call.
        let stop_result = if self.started {
            self.started = false;
            unsafe { self.client.Stop() }.map_err(AudioError::from)
        } else {
            Ok(())
        };
        let reset_result = unsafe { self.client.Reset() }.map_err(AudioError::from);
        stop_result.and(reset_result)
    }
}

impl Drop for SharedCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl SharedRender {
    /// Stop, release, refresh, and reopen the exact render binding with a
    /// bounded retry policy for transient device/service failures. The old
    /// client is released before any retry, and no substitute endpoint is
    /// ever selected.
    pub fn replace_with_refreshed_bound_with_retry(
        mut self,
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<Self, AudioError> {
        let stop_result = self.stop();
        drop(self);
        stop_result?;
        Self::open_refreshed_bound_with_retry(
            monitor,
            expected,
            buffer_duration_100ns,
            max_attempts,
            retry_delay_ms,
        )
    }

    /// Refresh and reopen the exact render binding with a bounded retry
    /// policy for transient device/service failures. This helper is intended
    /// for the control/recovery thread; it never sleeps on an audio callback
    /// and never selects a substitute endpoint.
    pub fn open_refreshed_bound_with_retry(
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
        max_attempts: u32,
        retry_delay_ms: u64,
    ) -> Result<Self, AudioError> {
        retry_transient_audio_operation(max_attempts, retry_delay_ms, || {
            Self::open_refreshed_bound(monitor, expected, buffer_duration_100ns)
        })
    }

    /// Stop and release this render client, refresh endpoint metadata, and
    /// open the same verified binding as a new client. The old client is
    /// always dropped before activation; endpoint changes fail closed.
    pub fn replace_with_refreshed_bound(
        mut self,
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
    ) -> Result<Self, AudioError> {
        let stop_result = self.stop();
        drop(self);
        stop_result?;
        Self::open_refreshed_bound(monitor, expected, buffer_duration_100ns)
    }

    /// Refresh endpoint metadata and then open only the exact persisted render
    /// binding. This is the preferred recovery entry point; a topology race
    /// after validation remains visible through the underlying WASAPI error.
    pub fn open_refreshed_bound(
        monitor: &mut EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
    ) -> Result<Self, AudioError> {
        monitor.refresh_changes()?;
        Self::open_bound(monitor, expected, buffer_duration_100ns)
    }

    /// Open an endpoint only when the monitor still reports the exact
    /// persisted ID, direction, and mix format. A stale binding fails before
    /// COM activation; endpoint changes after validation remain visible as
    /// the underlying WASAPI error.
    pub fn open_bound(
        monitor: &EndpointMonitor,
        expected: &EndpointInfo,
        buffer_duration_100ns: i64,
    ) -> Result<Self, AudioError> {
        let endpoint_id = bound_endpoint_id(monitor, expected, EndpointDirection::Render)?;
        Self::open(&endpoint_id, buffer_duration_100ns)
    }

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
        let event = EventHandle(unsafe {
            windows::Win32::System::Threading::CreateEventW(None, false, false, None)?
        });
        // Create the event before requesting the COM-allocated format so an
        // event-creation failure cannot leak the format buffer.
        let format = unsafe { client.GetMixFormat()? };
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
        initialized.map_err(|error| AudioError::WindowsOperation {
            operation: "IAudioClient::Initialize(render)",
            error,
        })?;
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
        let source_frames = u32::try_from(source.len() / bytes_per_frame)
            .map_err(|_| AudioError::InvalidFrameSize)?;
        let frames = available.min(source_frames);
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
        // Preserve best-effort reset semantics during teardown and avoid
        // retaining a stale started flag after any Stop failure.
        let stop_result = if self.started {
            self.started = false;
            unsafe { self.client.Stop() }.map_err(AudioError::from)
        } else {
            Ok(())
        };
        let reset_result = unsafe { self.client.Reset() }.map_err(AudioError::from);
        stop_result.and(reset_result)
    }
}

impl Drop for SharedRender {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Minimal output seam used by the bounded scheduler. The production
/// implementation is `SharedRender`; tests and a future driver harness may
/// provide a disposable sink without changing the scheduler's ownership or
/// realtime rules.
trait RenderSink {
    fn submit_bytes(&self, source: &[u8], bytes_per_frame: usize) -> Result<u32, AudioError>;
}

impl RenderSink for SharedRender {
    fn submit_bytes(&self, source: &[u8], bytes_per_frame: usize) -> Result<u32, AudioError> {
        SharedRender::submit_bytes(self, source, bytes_per_frame)
    }
}

fn retry_transient_audio_operation<T, F>(
    max_attempts: u32,
    retry_delay_ms: u64,
    mut operation: F,
) -> Result<T, AudioError>
where
    F: FnMut() -> Result<T, AudioError>,
{
    let attempts = max_attempts.clamp(1, 5);
    let delay = retry_delay_ms.min(1_000);
    for attempt in 0..attempts {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if attempt + 1 < attempts && error.is_retryable() => {
                if delay != 0 {
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                }
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("bounded audio retry loop always returns")
}

fn bound_endpoint_id(
    monitor: &EndpointMonitor,
    expected: &EndpointInfo,
    direction: EndpointDirection,
) -> Result<String, AudioError> {
    if expected.direction != direction {
        return Err(AudioError::EndpointBinding {
            resolution: Box::new(EndpointBindingResolution::DirectionChanged {
                id: expected.id.clone(),
                expected: direction,
                actual: expected.direction,
            }),
        });
    }
    match monitor.resolve_binding(expected) {
        EndpointBindingResolution::Available(actual) => Ok(actual.id),
        resolution => Err(AudioError::EndpointBinding {
            resolution: Box::new(resolution),
        }),
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
        self.refresh_changes()
    }

    /// Force a read-only endpoint resnapshot and return metadata changes.
    /// This is useful during recovery when a notification may have been
    /// coalesced or missed; it never opens a stream or selects a replacement.
    pub fn refresh_changes(&mut self) -> Result<Vec<EndpointChange>, AudioError> {
        self.notifications.take_dirty();
        let current = enumerate_active_endpoints()?;
        let changes = diff_endpoint_snapshots(&self.snapshot, &current);
        self.snapshot = current;
        Ok(changes)
    }

    pub fn snapshot(&self) -> &[EndpointInfo] {
        &self.snapshot
    }

    /// Resolve a persisted binding against the monitor's latest snapshot.
    /// This is observational only; callers must deliberately handle a
    /// non-available result before opening a replacement stream.
    pub fn resolve_binding(&self, expected: &EndpointInfo) -> EndpointBindingResolution {
        resolve_endpoint_binding_with_format(&self.snapshot, expected)
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

/// Enumerate the current Windows default endpoint for every supported role.
/// This is metadata-only: it does not activate an audio client, start a
/// stream, change defaults, or reserve an endpoint. A role with no assigned
/// device is omitted while the remaining roles are still reported.
pub fn enumerate_default_endpoint_bindings() -> Result<Vec<DefaultEndpointBinding>, AudioError> {
    unsafe {
        let initialized = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        initialized.ok()?;
        let result = enumerate_defaults_after_com_init();
        windows::Win32::System::Com::CoUninitialize();
        result
    }
}

/// Read bounded friendly names for the active endpoint snapshot. Names are
/// presentation only and never participate in endpoint identity or binding.
pub fn enumerate_active_endpoint_display_info() -> Result<Vec<EndpointDisplayInfo>, AudioError> {
    unsafe {
        let initialized = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        initialized.ok()?;
        let result = enumerate_display_info_after_com_init();
        windows::Win32::System::Com::CoUninitialize();
        result
    }
}

/// Enumerate endpoint identities and OS state for all known endpoint records.
/// This metadata-only inventory deliberately does not activate clients or
/// query mix formats, because disabled and unplugged devices may reject both.
pub fn enumerate_endpoint_states() -> Result<Vec<EndpointStateInfo>, AudioError> {
    unsafe {
        let initialized = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        initialized.ok()?;
        let result = enumerate_states_after_com_init();
        windows::Win32::System::Com::CoUninitialize();
        result
    }
}

/// Enumerate process identities suitable for a later process-loopback binding.
/// PID, executable name, verified executable path when available, and an
/// optional creation timestamp are returned; command lines and other process
/// details are intentionally excluded from this surface.
pub fn enumerate_applications() -> Result<Vec<ApplicationInfo>, AudioError> {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_core::PWSTR;

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
                if entry.th32ProcessID == 0 {
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                    continue;
                }
                let creation_time_100ns = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION,
                    false,
                    entry.th32ProcessID,
                )
                .ok()
                .map(|handle| {
                    let mut path_buffer = vec![0u16; 32_768];
                    let mut path_length = path_buffer.len() as u32;
                    let executable_path = QueryFullProcessImageNameW(
                        handle,
                        PROCESS_NAME_FORMAT(0),
                        PWSTR(path_buffer.as_mut_ptr()),
                        &mut path_length,
                    )
                    .ok()
                    .and_then(|_| String::from_utf16(&path_buffer[..path_length as usize]).ok());
                    let mut creation = FILETIME::default();
                    let mut exit = FILETIME::default();
                    let mut kernel = FILETIME::default();
                    let mut user = FILETIME::default();
                    let result =
                        GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user);
                    let _ = CloseHandle(handle);
                    let creation_time = result.ok().map(|_| {
                        (u64::from(creation.dwHighDateTime) << 32)
                            | u64::from(creation.dwLowDateTime)
                    });
                    (creation_time, executable_path)
                });
                let (creation_time_100ns, executable_path) =
                    creation_time_100ns.map_or((None, None), |(creation, path)| (creation, path));
                applications.push(ApplicationInfo {
                    process_id: entry.th32ProcessID,
                    executable,
                    executable_path,
                    creation_time_100ns,
                });
                if applications.len() >= MAX_APPLICATIONS
                    || Process32NextW(snapshot, &mut entry).is_err()
                {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        sort_application_inventory(&mut applications);
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
                                    render_session_count: 0,
                                    display_names: Vec::new(),
                                });
                        entry.total_session_count = entry.total_session_count.saturating_add(1);
                        if state == AudioSessionStateActive {
                            entry.active_session_count =
                                entry.active_session_count.saturating_add(1);
                        }
                        if is_capture {
                            entry.capture_session_count =
                                entry.capture_session_count.saturating_add(1);
                        } else {
                            entry.render_session_count =
                                entry.render_session_count.saturating_add(1);
                        }
                        let Ok(display_name) = session.GetDisplayName() else {
                            continue;
                        };
                        if !display_name.is_null() {
                            if let Ok(name) = display_name.to_string() {
                                retain_audio_display_name(&mut entry.display_names, name);
                            }
                            CoTaskMemFree(Some(display_name.0 as *const core::ffi::c_void));
                        }
                    }
                }
            }
            let mut inventory: Vec<_> = by_process.into_values().collect();
            sort_application_audio_inventory(&mut inventory);
            Ok(inventory)
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
    bind_application_with_path(
        process_id,
        expected_executable,
        None,
        expected_creation_time_100ns,
    )
}

/// Resolve an application using the complete observed executable identity.
/// Callers that persist a path should use this variant so two same-named
/// binaries in different locations cannot inherit one another's binding.
pub fn bind_application_with_path(
    process_id: u32,
    expected_executable: &str,
    expected_executable_path: Option<&str>,
    expected_creation_time_100ns: Option<u64>,
) -> Result<ApplicationInfo, AudioError> {
    if expected_creation_time_100ns.is_none() {
        return Err(AudioError::ApplicationIdentityUnavailable { process_id });
    }
    let application = enumerate_applications()?
        .into_iter()
        .find(|application| application.process_id == process_id)
        .ok_or(AudioError::ApplicationNotFound { process_id })?;
    let path_matches = expected_executable_path.map_or(true, |expected| {
        application
            .executable_path
            .as_deref()
            .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
    });
    if !application
        .executable
        .eq_ignore_ascii_case(expected_executable)
        || !path_matches
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
    resolve_application_restart_with_path(applications, expected_executable, None)
}

/// Resolve a persisted restart selector using basename, optional full path,
/// and creation identity. A supplied path is required to be observed exactly
/// (case-insensitively) before a candidate can be returned.
pub fn resolve_application_restart_with_path(
    applications: &[ApplicationInfo],
    expected_executable: &str,
    expected_executable_path: Option<&str>,
) -> Result<ApplicationInfo, AudioError> {
    let matches = applications
        .iter()
        .filter(|application| {
            application
                .executable
                .eq_ignore_ascii_case(expected_executable)
                && expected_executable_path.map_or(true, |expected| {
                    application
                        .executable_path
                        .as_deref()
                        .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
                })
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
        DEVICE_STATE_ACTIVE, WAVEFORMATEXTENSIBLE,
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
            let (channel_mask, subformat_guid) = if format_value.wFormatTag == 0xfffe
                && format_value.cbSize >= 22
            {
                let extensible = std::ptr::read_unaligned(format.cast::<WAVEFORMATEXTENSIBLE>());
                let guid = std::ptr::read_unaligned(std::ptr::addr_of!(extensible.SubFormat));
                (extensible.dwChannelMask, format!("{guid:?}"))
            } else {
                (0, String::new())
            };
            endpoints.push(EndpointInfo {
                id,
                direction,
                default_period_100ns: default_period,
                minimum_period_100ns: minimum_period,
                sample_rate_hz: format_value.nSamplesPerSec,
                channels: format_value.nChannels,
                bits_per_sample: format_value.wBitsPerSample,
                format_tag: format_value.wFormatTag,
                channel_mask,
                subformat_guid,
            });
            windows::Win32::System::Com::CoTaskMemFree(Some(format.cast()));
        }
    }
    Ok(endpoints)
}

unsafe fn enumerate_defaults_after_com_init() -> Result<Vec<DefaultEndpointBinding>, AudioError> {
    use windows::Win32::Media::Audio::{
        eCapture, eCommunications, eConsole, eMultimedia, eRender, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let mut bindings = Vec::new();
    for (direction, flow) in [
        (EndpointDirection::Capture, eCapture),
        (EndpointDirection::Render, eRender),
    ] {
        for (role, windows_role) in [
            (DefaultEndpointRole::Console, eConsole),
            (DefaultEndpointRole::Multimedia, eMultimedia),
            (DefaultEndpointRole::Communications, eCommunications),
        ] {
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(flow, windows_role) else {
                continue;
            };
            let id = device
                .GetId()
                .map_err(AudioError::Windows)?
                .to_string()
                .map_err(|_| AudioError::InvalidUtf16)?;
            bindings.push(DefaultEndpointBinding {
                direction,
                role,
                endpoint_id: id,
            });
        }
    }
    Ok(bindings)
}

unsafe fn enumerate_display_info_after_com_init() -> Result<Vec<EndpointDisplayInfo>, AudioError> {
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{
        eCapture, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATEMASK_ALL,
    };
    use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToString};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ};

    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let mut result = Vec::new();
    for (direction, flow) in [
        (EndpointDirection::Capture, eCapture),
        (EndpointDirection::Render, eRender),
    ] {
        let devices = enumerator.EnumAudioEndpoints(
            flow,
            windows::Win32::Media::Audio::DEVICE_STATE(DEVICE_STATEMASK_ALL),
        )?;
        for index in 0..devices.GetCount()? {
            let device = devices.Item(index)?;
            let id = device
                .GetId()?
                .to_string()
                .map_err(|_| AudioError::InvalidUtf16)?;
            let store = device.OpenPropertyStore(STGM_READ)?;
            let mut value = store.GetValue(&PKEY_Device_FriendlyName)?;
            let mut buffer = [0_u16; 512];
            let name = PropVariantToString(&value, &mut buffer)
                .ok()
                .and_then(|_| {
                    let length = buffer.iter().position(|character| *character == 0)?;
                    String::from_utf16(&buffer[..length]).ok()
                })
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Unknown audio endpoint".to_owned());
            PropVariantClear(&mut value).ok();
            result.push(EndpointDisplayInfo {
                id,
                direction,
                name,
            });
        }
    }
    Ok(result)
}

unsafe fn enumerate_states_after_com_init() -> Result<Vec<EndpointStateInfo>, AudioError> {
    use windows::Win32::Media::Audio::{
        eCapture, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATEMASK_ALL,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let mut result = Vec::new();
    for (direction, flow) in [
        (EndpointDirection::Capture, eCapture),
        (EndpointDirection::Render, eRender),
    ] {
        let devices = enumerator.EnumAudioEndpoints(
            flow,
            windows::Win32::Media::Audio::DEVICE_STATE(DEVICE_STATEMASK_ALL),
        )?;
        for index in 0..devices.GetCount()? {
            let device = devices.Item(index)?;
            let id = device
                .GetId()?
                .to_string()
                .map_err(|_| AudioError::InvalidUtf16)?;
            let raw_state = device.GetState()?.0;
            let state = endpoint_state_from_raw(raw_state);
            result.push(EndpointStateInfo {
                id,
                direction,
                state,
            });
        }
    }
    Ok(result)
}

const BRIDGE_STATE_OFFSET: usize = 0;
const BRIDGE_HEADER_OFFSET: usize = 8;
const BRIDGE_HEADER_BYTES: usize = 24;
const BRIDGE_PAYLOAD_OFFSET: usize = BRIDGE_HEADER_OFFSET + BRIDGE_HEADER_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeBridgeRegionError {
    InvalidPath,
    Exists,
    BufferTooSmall,
    Busy,
    Empty,
    TornRead,
    StaleGeneration,
    SequenceRegression,
    InvalidFrame,
    Io(String),
    Contract(audiorouter_protocol::AudioBridgeContractError),
}

/// One bounded, file-backed shared-memory slot for the future driver broker.
/// The file path is supplied by the authorized broker and is never chosen from
/// a display label. The mapping is control/broker-side; realtime callers should
/// receive a prevalidated handle and use only nonblocking slot operations.
pub struct NativeBridgeRegion {
    // Retained to keep the mapped file alive for the view's lifetime.
    _file: std::fs::File,
    map: memmap2::MmapMut,
    channels: u16,
    max_frames: u16,
}

impl NativeBridgeRegion {
    pub fn create(
        path: impl AsRef<std::path::Path>,
        channels: u16,
        max_frames: u16,
    ) -> Result<Self, NativeBridgeRegionError> {
        let path = path.as_ref();
        Self::validate_layout(path, channels, max_frames)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    NativeBridgeRegionError::Exists
                } else {
                    NativeBridgeRegionError::Io(error.to_string())
                }
            })?;
        let length = Self::buffer_len(channels, max_frames);
        file.set_len(length as u64)
            .map_err(|error| NativeBridgeRegionError::Io(error.to_string()))?;
        // SAFETY: the file is resized to the exact mapping length immediately
        // above and the mapping is kept alive by `self.file` for its lifetime.
        let map = unsafe { memmap2::MmapOptions::new().len(length).map_mut(&file) }
            .map_err(|error| NativeBridgeRegionError::Io(error.to_string()))?;
        Ok(Self {
            _file: file,
            map,
            channels,
            max_frames,
        })
    }

    pub fn open(
        path: impl AsRef<std::path::Path>,
        channels: u16,
        max_frames: u16,
    ) -> Result<Self, NativeBridgeRegionError> {
        let path = path.as_ref();
        Self::validate_layout(path, channels, max_frames)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|error| NativeBridgeRegionError::Io(error.to_string()))?;
        let length = Self::buffer_len(channels, max_frames);
        if file
            .metadata()
            .map_err(|error| NativeBridgeRegionError::Io(error.to_string()))?
            .len()
            < length as u64
        {
            return Err(NativeBridgeRegionError::BufferTooSmall);
        }
        // SAFETY: the existing file was checked to be at least `length` bytes;
        // the file handle remains owned by this region while the view exists.
        let map = unsafe { memmap2::MmapOptions::new().len(length).map_mut(&file) }
            .map_err(|error| NativeBridgeRegionError::Io(error.to_string()))?;
        Ok(Self {
            _file: file,
            map,
            channels,
            max_frames,
        })
    }

    pub fn mapping_bytes(&self) -> usize {
        Self::buffer_len(self.channels, self.max_frames)
    }

    pub fn write(
        &self,
        generation: u64,
        sequence: u64,
        samples: &[f32],
    ) -> Result<(), NativeBridgeRegionError> {
        if samples.is_empty() || samples.len() % usize::from(self.channels) != 0 {
            return Err(NativeBridgeRegionError::InvalidFrame);
        }
        let frames = samples.len() / usize::from(self.channels);
        if frames > usize::from(self.max_frames) {
            return Err(NativeBridgeRegionError::InvalidFrame);
        }
        let payload_bytes = samples
            .len()
            .checked_mul(std::mem::size_of::<f32>())
            .and_then(|length| u32::try_from(length).ok())
            .ok_or(NativeBridgeRegionError::InvalidFrame)?;
        let header = audiorouter_protocol::AudioBridgeBlockHeader {
            generation,
            sequence,
            frames: frames as u16,
            channels: self.channels,
            payload_bytes,
        };
        header
            .validate()
            .map_err(NativeBridgeRegionError::Contract)?;
        let state = self.state() as *const std::sync::atomic::AtomicU64;
        // SAFETY: `state` points into this region's live, aligned mapping and
        // remains valid for the duration of the write operation.
        let current = unsafe { (*state).load(std::sync::atomic::Ordering::Acquire) };
        if current & 1 != 0 {
            return Err(NativeBridgeRegionError::Busy);
        }
        if current != 0 {
            let previous = u64::from_le_bytes(
                self.map[BRIDGE_HEADER_OFFSET + 8..BRIDGE_HEADER_OFFSET + 16]
                    .try_into()
                    .unwrap(),
            );
            if sequence <= previous {
                return Err(NativeBridgeRegionError::SequenceRegression);
            }
        }
        // SAFETY: see the state-pointer invariant above; the atomic operation
        // coordinates readers of the same mapped slot.
        unsafe {
            (*state).compare_exchange(
                current,
                current.saturating_add(1),
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
        }
        .map_err(|_| NativeBridgeRegionError::Busy)?;
        // SAFETY: the successful state CAS grants this producer exclusive
        // access to the payload until the release store below. The mapping
        // remains alive through `self`, and all offsets are layout-checked.
        let map =
            unsafe { std::slice::from_raw_parts_mut(self.map.as_ptr() as *mut u8, self.map.len()) };
        map[BRIDGE_HEADER_OFFSET..BRIDGE_HEADER_OFFSET + 8]
            .copy_from_slice(&header.generation.to_le_bytes());
        map[BRIDGE_HEADER_OFFSET + 8..BRIDGE_HEADER_OFFSET + 16]
            .copy_from_slice(&header.sequence.to_le_bytes());
        map[BRIDGE_HEADER_OFFSET + 16..BRIDGE_HEADER_OFFSET + 18]
            .copy_from_slice(&header.frames.to_le_bytes());
        map[BRIDGE_HEADER_OFFSET + 18..BRIDGE_HEADER_OFFSET + 20]
            .copy_from_slice(&header.channels.to_le_bytes());
        map[BRIDGE_HEADER_OFFSET + 20..BRIDGE_HEADER_OFFSET + 24]
            .copy_from_slice(&header.payload_bytes.to_le_bytes());
        for (index, sample) in samples.iter().enumerate() {
            let offset = BRIDGE_PAYLOAD_OFFSET + index * 4;
            map[offset..offset + 4].copy_from_slice(&sample.to_le_bytes());
        }
        // SAFETY: the mapping remains owned by `self` until after this store.
        unsafe {
            (*state).store(
                current.saturating_add(2),
                std::sync::atomic::Ordering::Release,
            )
        };
        Ok(())
    }

    pub fn read_into(
        &self,
        expected_generation: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeRegionError> {
        self.read_into_after(expected_generation, 0, samples)
    }

    /// Read only a block newer than the caller's last consumed sequence.
    /// Supplying the floor is required for replay-safe render consumption.
    pub fn read_into_after(
        &self,
        expected_generation: u64,
        minimum_sequence: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeRegionError> {
        let state = self.state();
        let before = state.load(std::sync::atomic::Ordering::Acquire);
        if before == 0 {
            return Err(NativeBridgeRegionError::Empty);
        }
        if before & 1 != 0 {
            return Err(NativeBridgeRegionError::Busy);
        }
        let header = self.header();
        header
            .validate()
            .map_err(NativeBridgeRegionError::Contract)?;
        if header.generation != expected_generation {
            return Err(NativeBridgeRegionError::StaleGeneration);
        }
        if header.sequence <= minimum_sequence {
            return Err(NativeBridgeRegionError::SequenceRegression);
        }
        let sample_count = usize::from(header.frames) * usize::from(header.channels);
        if samples.len() < sample_count {
            return Err(NativeBridgeRegionError::BufferTooSmall);
        }
        for (index, destination) in samples[..sample_count].iter_mut().enumerate() {
            let offset = BRIDGE_PAYLOAD_OFFSET + index * 4;
            *destination = f32::from_le_bytes(self.map[offset..offset + 4].try_into().unwrap());
            if !destination.is_finite() {
                return Err(NativeBridgeRegionError::InvalidFrame);
            }
        }
        if state.load(std::sync::atomic::Ordering::Acquire) != before {
            return Err(NativeBridgeRegionError::TornRead);
        }
        Ok(header)
    }

    pub fn flush(&mut self) -> Result<(), NativeBridgeRegionError> {
        self.map
            .flush()
            .map_err(|error| NativeBridgeRegionError::Io(error.to_string()))
    }

    fn validate_layout(
        path: &std::path::Path,
        channels: u16,
        max_frames: u16,
    ) -> Result<(), NativeBridgeRegionError> {
        if !path.is_absolute() || path.as_os_str().is_empty() {
            return Err(NativeBridgeRegionError::InvalidPath);
        }
        let mut current = path.parent();
        while let Some(parent) = current {
            if parent.exists()
                && std::fs::symlink_metadata(parent)
                    .map(|metadata| metadata.file_type().is_symlink())
                    .unwrap_or(true)
            {
                return Err(NativeBridgeRegionError::InvalidPath);
            }
            current = parent.parent();
        }
        if !(1..=audiorouter_protocol::MAX_AUDIO_BRIDGE_CHANNELS).contains(&channels)
            || !(1..=audiorouter_protocol::MAX_AUDIO_BRIDGE_FRAMES).contains(&max_frames)
        {
            return Err(NativeBridgeRegionError::InvalidFrame);
        }
        Ok(())
    }

    fn buffer_len(channels: u16, max_frames: u16) -> usize {
        BRIDGE_PAYLOAD_OFFSET
            + usize::from(channels) * usize::from(max_frames) * std::mem::size_of::<f32>()
    }

    fn state(&self) -> &std::sync::atomic::AtomicU64 {
        // SAFETY: the mapping is at least `BRIDGE_PAYLOAD_OFFSET` bytes and
        // the state offset is naturally aligned for AtomicU64.
        unsafe {
            &*(self.map.as_ptr().add(BRIDGE_STATE_OFFSET) as *const std::sync::atomic::AtomicU64)
        }
    }

    fn header(&self) -> audiorouter_protocol::AudioBridgeBlockHeader {
        audiorouter_protocol::AudioBridgeBlockHeader {
            generation: u64::from_le_bytes(
                self.map[BRIDGE_HEADER_OFFSET..BRIDGE_HEADER_OFFSET + 8]
                    .try_into()
                    .unwrap(),
            ),
            sequence: u64::from_le_bytes(
                self.map[BRIDGE_HEADER_OFFSET + 8..BRIDGE_HEADER_OFFSET + 16]
                    .try_into()
                    .unwrap(),
            ),
            frames: u16::from_le_bytes(
                self.map[BRIDGE_HEADER_OFFSET + 16..BRIDGE_HEADER_OFFSET + 18]
                    .try_into()
                    .unwrap(),
            ),
            channels: u16::from_le_bytes(
                self.map[BRIDGE_HEADER_OFFSET + 18..BRIDGE_HEADER_OFFSET + 20]
                    .try_into()
                    .unwrap(),
            ),
            payload_bytes: u32::from_le_bytes(
                self.map[BRIDGE_HEADER_OFFSET + 20..BRIDGE_HEADER_OFFSET + 24]
                    .try_into()
                    .unwrap(),
            ),
        }
    }
}

/// Allocation-free `AudioTap` adapter for the broker's single mapped slot.
///
/// The scratch buffer is allocated at construction. A single engine callback
/// owns a writer in normal operation; the atomic guard makes an accidental
/// concurrent callback fail closed instead of racing the scratch buffer.
pub struct NativeBridgeRealtimeWriter {
    region: std::cell::UnsafeCell<NativeBridgeRegion>,
    generation: u64,
    next_sequence: AtomicU64,
    published_blocks: AtomicU64,
    in_use: AtomicBool,
    scratch: std::cell::UnsafeCell<Vec<f32>>,
}

// The writer's Rust mapping and scratch are exclusively owned by the writer;
// `in_use` admits only one callback at a time. The other mapping participant
// is the external kernel reader, coordinated by the bridge seqlock protocol.
unsafe impl Send for NativeBridgeRealtimeWriter {}
unsafe impl Sync for NativeBridgeRealtimeWriter {}

impl NativeBridgeRealtimeWriter {
    pub fn new(
        region: NativeBridgeRegion,
        generation: u64,
    ) -> Result<Self, NativeBridgeRegionError> {
        if generation == 0 {
            return Err(NativeBridgeRegionError::InvalidFrame);
        }
        let sample_count = usize::from(region.channels) * usize::from(region.max_frames);
        Ok(Self {
            region: std::cell::UnsafeCell::new(region),
            generation,
            next_sequence: AtomicU64::new(0),
            published_blocks: AtomicU64::new(0),
            in_use: AtomicBool::new(false),
            scratch: std::cell::UnsafeCell::new(vec![0.0; sample_count]),
        })
    }

    pub fn published_blocks(&self) -> u64 {
        self.published_blocks.load(Ordering::Relaxed)
    }
}

impl audiorouter_engine::AudioTap for NativeBridgeRealtimeWriter {
    fn on_processed_block(&self, _start_frame: u64, block: &audiorouter_engine::AudioBlock) {
        if self
            .in_use
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return;
        }
        (|| {
            // SAFETY: `in_use` gives this callback exclusive access to the
            // writer-owned mapping until the guard is released.
            let region = unsafe { &*self.region.get() };
            if block.channels() != usize::from(region.channels)
                || block.frames() > usize::from(region.max_frames)
            {
                return;
            }
            // SAFETY: `in_use` gives this callback exclusive access to the
            // preallocated scratch vector until the guard is released.
            let scratch = unsafe { &mut *self.scratch.get() };
            let count = block.channels() * block.frames();
            for frame in 0..block.frames() {
                for channel in 0..block.channels() {
                    scratch[frame * block.channels() + channel] =
                        block.channel(channel).unwrap()[frame];
                }
            }
            let Some(sequence) = self
                .next_sequence
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(1)
                })
                .ok()
                .and_then(|value| value.checked_add(1))
            else {
                return;
            };
            if region
                .write(self.generation, sequence, &scratch[..count])
                .is_ok()
            {
                self.published_blocks.fetch_add(1, Ordering::Relaxed);
            }
        })();
        self.in_use.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeBridgeSessionError {
    InvalidHello(audiorouter_protocol::AudioBridgeContractError),
    Region(NativeBridgeRegionError),
    SequenceExhausted,
    LeaseExpired,
    WrongDirection,
}

/// Negotiated owner of one native bridge slot.
///
/// The session keeps the protocol identity next to the mapped region so a
/// caller cannot accidentally write a valid block for another bus or graph
/// generation. Construction and opening are control-plane operations;
/// `write` and `read_into` only use caller-owned buffers and the bounded slot.
pub struct NativeBridgeSession {
    hello: audiorouter_protocol::AudioBridgeHello,
    mapping_path: std::path::PathBuf,
    region: NativeBridgeRegion,
    next_sequence: u64,
    last_heartbeat: std::time::Instant,
}

impl NativeBridgeSession {
    pub fn create(
        path: impl AsRef<std::path::Path>,
        hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<Self, NativeBridgeSessionError> {
        hello
            .validate()
            .map_err(NativeBridgeSessionError::InvalidHello)?;
        let mapping_path = path.as_ref().to_path_buf();
        let region =
            NativeBridgeRegion::create(&mapping_path, hello.channels, hello.frames_per_quantum)
                .map_err(NativeBridgeSessionError::Region)?;
        Ok(Self {
            hello,
            mapping_path,
            region,
            next_sequence: 0,
            last_heartbeat: std::time::Instant::now(),
        })
    }

    pub fn open(
        path: impl AsRef<std::path::Path>,
        hello: audiorouter_protocol::AudioBridgeHello,
    ) -> Result<Self, NativeBridgeSessionError> {
        hello
            .validate()
            .map_err(NativeBridgeSessionError::InvalidHello)?;
        let mapping_path = path.as_ref().to_path_buf();
        let region =
            NativeBridgeRegion::open(&mapping_path, hello.channels, hello.frames_per_quantum)
                .map_err(NativeBridgeSessionError::Region)?;
        Ok(Self {
            hello,
            mapping_path,
            region,
            next_sequence: 0,
            last_heartbeat: std::time::Instant::now(),
        })
    }

    pub fn hello(&self) -> &audiorouter_protocol::AudioBridgeHello {
        &self.hello
    }

    pub fn mapping_bytes(&self) -> usize {
        NativeBridgeRegion::buffer_len(self.hello.channels, self.hello.frames_per_quantum)
    }

    /// Open a separate producer mapping after negotiation. Preparation may
    /// allocate and map; the returned writer's tap callback is bounded and
    /// nonblocking.
    pub fn realtime_writer(&self) -> Result<NativeBridgeRealtimeWriter, NativeBridgeSessionError> {
        if self.hello.direction != audiorouter_protocol::AudioBridgeDirection::CaptureSink {
            return Err(NativeBridgeSessionError::WrongDirection);
        }
        let region = NativeBridgeRegion::open(
            &self.mapping_path,
            self.hello.channels,
            self.hello.frames_per_quantum,
        )
        .map_err(NativeBridgeSessionError::Region)?;
        NativeBridgeRealtimeWriter::new(region, self.hello.generation)
            .map_err(NativeBridgeSessionError::Region)
    }

    pub fn write(&mut self, samples: &[f32]) -> Result<u64, NativeBridgeSessionError> {
        self.write_at(std::time::Instant::now(), samples)
    }

    fn write_at(
        &mut self,
        now: std::time::Instant,
        samples: &[f32],
    ) -> Result<u64, NativeBridgeSessionError> {
        if self.hello.direction != audiorouter_protocol::AudioBridgeDirection::CaptureSink {
            return Err(NativeBridgeSessionError::WrongDirection);
        }
        self.ensure_lease_at(now)?;
        let sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(NativeBridgeSessionError::SequenceExhausted)?;
        self.region
            .write(self.hello.generation, sequence, samples)
            .map_err(NativeBridgeSessionError::Region)?;
        self.next_sequence = sequence;
        Ok(sequence)
    }

    pub fn read_into(
        &self,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeSessionError> {
        self.read_into_after(0, samples)
    }

    /// Read a render-source block newer than the caller's last consumed
    /// sequence. This makes reconnect/polling replay policy explicit.
    pub fn read_into_after(
        &self,
        minimum_sequence: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeSessionError> {
        self.read_into_at(std::time::Instant::now(), minimum_sequence, samples)
    }

    fn read_into_at(
        &self,
        now: std::time::Instant,
        minimum_sequence: u64,
        samples: &mut [f32],
    ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeSessionError> {
        if self.hello.direction != audiorouter_protocol::AudioBridgeDirection::RenderSource {
            return Err(NativeBridgeSessionError::WrongDirection);
        }
        self.ensure_lease_at(now)?;
        self.region
            .read_into_after(self.hello.generation, minimum_sequence, samples)
            .map_err(NativeBridgeSessionError::Region)
    }

    pub fn flush(&mut self) -> Result<(), NativeBridgeSessionError> {
        self.region
            .flush()
            .map_err(NativeBridgeSessionError::Region)
    }

    pub fn heartbeat(&mut self) -> Result<(), NativeBridgeSessionError> {
        self.heartbeat_at(std::time::Instant::now())
    }

    pub fn heartbeat_at(
        &mut self,
        now: std::time::Instant,
    ) -> Result<(), NativeBridgeSessionError> {
        self.ensure_lease_at(now)?;
        self.last_heartbeat = now;
        Ok(())
    }

    pub fn is_lease_expired_at(&self, now: std::time::Instant) -> bool {
        self.lease_elapsed_at(now).is_some_and(|elapsed| {
            elapsed > std::time::Duration::from_millis(u64::from(self.hello.lease_ms))
        })
    }

    fn ensure_lease_at(&self, now: std::time::Instant) -> Result<(), NativeBridgeSessionError> {
        if self.is_lease_expired_at(now) {
            Err(NativeBridgeSessionError::LeaseExpired)
        } else {
            Ok(())
        }
    }

    fn lease_elapsed_at(&self, now: std::time::Instant) -> Option<std::time::Duration> {
        now.checked_duration_since(self.last_heartbeat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn native_render_source_transient_read_failures_fail_closed_to_silence() {
        let transient = [
            NativeBridgeRegionError::Empty,
            NativeBridgeRegionError::Busy,
            NativeBridgeRegionError::TornRead,
            NativeBridgeRegionError::SequenceRegression,
            NativeBridgeRegionError::StaleGeneration,
        ];
        for region_error in transient {
            assert!(native_bridge_input_error_is_silence(
                &NativeBridgeControllerError::Session(NativeBridgeSessionError::Region(
                    region_error,
                ))
            ));
        }
        assert!(!native_bridge_input_error_is_silence(
            &NativeBridgeControllerError::Session(NativeBridgeSessionError::LeaseExpired)
        ));
        assert!(!native_bridge_input_error_is_silence(
            &NativeBridgeControllerError::Session(NativeBridgeSessionError::SequenceExhausted)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn native_render_source_pump_accepts_injected_blocks_and_rejects_replays() {
        struct Source {
            replay: bool,
        }

        impl NativeRenderSource for Source {
            fn read_into_after(
                &self,
                _minimum_sequence: u64,
                samples: &mut [f32],
            ) -> Result<audiorouter_protocol::AudioBridgeBlockHeader, NativeBridgeControllerError>
            {
                if self.replay {
                    return Err(NativeBridgeControllerError::Session(
                        NativeBridgeSessionError::Region(
                            NativeBridgeRegionError::SequenceRegression,
                        ),
                    ));
                }
                samples[..128].fill(0.25);
                Ok(audiorouter_protocol::AudioBridgeBlockHeader {
                    generation: 1,
                    sequence: 7,
                    frames: 64,
                    channels: 2,
                    payload_bytes: 512,
                })
            }
        }

        struct Sink;
        impl RenderSink for Sink {
            fn submit_bytes(
                &self,
                _source: &[u8],
                _bytes_per_frame: usize,
            ) -> Result<u32, AudioError> {
                Ok(0)
            }
        }

        let mut bridge = WasapiSchedulerBridge::new(2, 2, 64, 64).unwrap();
        let mut last_sequence = 0;
        let result = bridge
            .pump_from_native_render_source(&Source { replay: false }, &Sink, &mut last_sequence)
            .unwrap();
        assert_eq!(result.packets, 1);
        assert_eq!(result.processed_quanta, 1);
        assert_eq!(last_sequence, 7);

        let result = bridge
            .pump_from_native_render_source(&Source { replay: true }, &Sink, &mut last_sequence)
            .unwrap();
        assert_eq!(result.packets, 1);
        assert_eq!(result.processed_quanta, 1);
        assert_eq!(last_sequence, 7);
    }

    #[test]
    fn injected_endpoint_lifecycle_rolls_back_and_stops_both_clients() {
        struct Probe {
            starts: u8,
            stops: u8,
            fail_start: bool,
            fail_stop: bool,
        }

        impl EndpointLifecycle for Probe {
            fn start(&mut self) -> Result<(), AudioError> {
                self.starts += 1;
                if self.fail_start {
                    Err(AudioError::ProcessingStateUnavailable)
                } else {
                    Ok(())
                }
            }

            fn stop(&mut self) -> Result<(), AudioError> {
                self.stops += 1;
                if self.fail_stop {
                    Err(AudioError::ProcessingStateUnavailable)
                } else {
                    Ok(())
                }
            }
        }

        let mut capture = Probe {
            starts: 0,
            stops: 0,
            fail_start: false,
            fail_stop: false,
        };
        let mut render = Probe {
            starts: 0,
            stops: 0,
            fail_start: true,
            fail_stop: false,
        };
        assert!(start_endpoint_pair(&mut capture, &mut render).is_err());
        assert_eq!((capture.starts, capture.stops), (1, 1));
        assert_eq!(render.starts, 1);

        render.fail_start = false;
        render.fail_stop = true;
        assert!(start_endpoint_pair(&mut capture, &mut render).is_ok());
        assert_eq!((capture.starts, render.starts), (2, 2));
        let mut reset_called = false;
        let stop_error = stop_endpoint_pair_and_reset(&mut capture, &mut render, || {
            reset_called = true;
            Err(AudioError::InvalidFrameSize)
        })
        .unwrap_err();
        assert!(matches!(stop_error, AudioError::ProcessingStateUnavailable));
        assert!(reset_called);
        assert_eq!((capture.stops, render.stops), (2, 1));
    }

    #[test]
    fn worker_packet_budget_and_telemetry_accumulation_are_bounded() {
        assert_eq!(MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE, 64);
        let mut total = WasapiSchedulerPump {
            packets: u32::MAX,
            captured_frames: u32::MAX,
            processed_quanta: u32::MAX,
            rendered_frames: u32::MAX,
            dropped_render_frames: u32::MAX,
        };
        total.accumulate(WasapiSchedulerPump {
            packets: 1,
            captured_frames: 1,
            processed_quanta: 1,
            rendered_frames: 1,
            dropped_render_frames: 1,
        });
        assert_eq!(total.packets, u32::MAX);
        assert_eq!(total.captured_frames, u32::MAX);
        assert_eq!(total.processed_quanta, u32::MAX);
        assert_eq!(total.rendered_frames, u32::MAX);
        assert_eq!(total.dropped_render_frames, u32::MAX);
    }

    #[test]
    fn float32_boundary_converts_interleaved_and_planar_without_allocating() {
        let mut block = audiorouter_engine::AudioBlock::new(2, 2).unwrap();
        let source = [0.25_f32, -0.5, 1.0, f32::NAN];
        let bytes: Vec<u8> = source
            .iter()
            .flat_map(|sample| sample.to_ne_bytes())
            .collect();
        decode_interleaved_float32(&bytes, 2, &mut block).unwrap();
        assert_eq!(block.channel(0).unwrap(), &[0.25, 1.0]);
        assert_eq!(block.channel(1).unwrap(), &[-0.5, 0.0]);

        let mut encoded = vec![0_u8; bytes.len()];
        encode_interleaved_float32(&block, &mut encoded).unwrap();
        let encoded_samples: Vec<f32> = encoded
            .chunks_exact(4)
            .map(|sample| f32::from_ne_bytes(sample.try_into().unwrap()))
            .collect();
        assert_eq!(encoded_samples, vec![0.25, -0.5, 1.0, 0.0]);
    }

    #[test]
    fn float32_boundary_rejects_wrong_shape_before_access() {
        let mut block = audiorouter_engine::AudioBlock::new(2, 2).unwrap();
        assert!(matches!(
            decode_interleaved_float32(&[0; 4], 2, &mut block),
            Err(AudioError::BufferTooSmall { .. })
        ));
        assert!(matches!(
            encode_interleaved_float32(&block, &mut [0; 4]),
            Err(AudioError::BufferTooSmall { .. })
        ));
    }

    #[test]
    fn default_endpoint_roles_have_stable_contract_names() {
        assert_eq!(
            DefaultEndpointRole::ALL
                .iter()
                .map(|role| role.as_str())
                .collect::<Vec<_>>(),
            vec!["console", "multimedia", "communications"]
        );
    }

    #[test]
    fn endpoint_state_mapping_preserves_unknown_os_values() {
        assert_eq!(endpoint_state_from_raw(1), EndpointState::Active);
        assert_eq!(endpoint_state_from_raw(2), EndpointState::Disabled);
        assert_eq!(endpoint_state_from_raw(4), EndpointState::NotPresent);
        assert_eq!(endpoint_state_from_raw(8), EndpointState::Unplugged);
        assert_eq!(endpoint_state_from_raw(16), EndpointState::Unknown(16));
    }

    #[test]
    fn endpoint_state_diff_covers_inactive_activation_and_removal() {
        let state = |id: &str, value| EndpointStateInfo {
            id: id.into(),
            direction: EndpointDirection::Capture,
            state: value,
        };
        let before = [
            state("reconnect", EndpointState::Unplugged),
            state("gone", EndpointState::Disabled),
        ];
        let current = [
            state("reconnect", EndpointState::Active),
            state("new", EndpointState::Active),
        ];
        assert_eq!(
            diff_endpoint_state_snapshots(&before, &current),
            vec![
                EndpointStateChange::Removed(state("gone", EndpointState::Disabled)),
                EndpointStateChange::Changed {
                    before: state("reconnect", EndpointState::Unplugged),
                    after: state("reconnect", EndpointState::Active),
                },
                EndpointStateChange::Added(state("new", EndpointState::Active)),
            ]
        );
    }

    #[test]
    fn packet_accumulator_splits_and_reassembles_variable_packets() {
        let mut accumulator = Float32PacketAccumulator::new(2, 2, 4).unwrap();
        let mut block = audiorouter_engine::AudioBlock::new(2, 2).unwrap();
        let first = [0.0_f32, 0.1, 1.0, 1.1, 2.0, 2.1];
        let first_bytes: Vec<u8> = first
            .iter()
            .flat_map(|sample| sample.to_ne_bytes())
            .collect();
        assert_eq!(accumulator.push(&first_bytes).unwrap(), first_bytes.len());
        assert_eq!(accumulator.pending_frames(), 3);
        assert!(accumulator.pop_into(&mut block).unwrap());
        assert_eq!(block.channel(0).unwrap(), &[0.0, 1.0]);
        assert_eq!(block.channel(1).unwrap(), &[0.1, 1.1]);
        assert_eq!(accumulator.pending_frames(), 1);

        let second = [3.0_f32, 3.1];
        let second_bytes: Vec<u8> = second
            .iter()
            .flat_map(|sample| sample.to_ne_bytes())
            .collect();
        assert_eq!(accumulator.push(&second_bytes).unwrap(), second_bytes.len());
        assert!(accumulator.pop_into(&mut block).unwrap());
        assert_eq!(block.channel(0).unwrap(), &[2.0, 3.0]);
        assert_eq!(block.channel(1).unwrap(), &[2.1, 3.1]);
        assert_eq!(accumulator.pending_frames(), 0);
        assert!(!accumulator.pop_into(&mut block).unwrap());
    }

    #[test]
    fn packet_accumulator_reports_backpressure_without_dropping_source_shape() {
        let mut accumulator = Float32PacketAccumulator::new(1, 2, 2).unwrap();
        let source = [0.0_f32, 1.0, 2.0, 3.0];
        let bytes: Vec<u8> = source
            .iter()
            .flat_map(|sample| sample.to_ne_bytes())
            .collect();
        assert_eq!(accumulator.push(&bytes).unwrap(), 8);
        assert_eq!(accumulator.pending_frames(), 2);
        assert_eq!(accumulator.push(&bytes[8..]).unwrap(), 0);
        assert!(matches!(
            accumulator.push(&[0; 3]),
            Err(AudioError::InvalidFrameSize)
        ));
    }

    #[test]
    fn packet_accumulator_reset_discards_partial_endpoint_audio() {
        let mut accumulator = Float32PacketAccumulator::new(1, 4, 8).unwrap();
        let partial = [0.25_f32, 0.5];
        let bytes: Vec<u8> = partial
            .iter()
            .flat_map(|sample| sample.to_ne_bytes())
            .collect();
        accumulator.push(&bytes).unwrap();
        assert_eq!(accumulator.pending_frames(), 2);
        accumulator.reset();
        assert_eq!(accumulator.pending_frames(), 0);
        let mut block = audiorouter_engine::AudioBlock::new(1, 4).unwrap();
        assert!(!accumulator.pop_into(&mut block).unwrap());
    }

    #[test]
    fn scheduler_bridge_reset_stream_recycles_queued_audio_and_keeps_graph() {
        let mut bridge = WasapiSchedulerBridge::new(2, 1, 2, 4).unwrap();
        let generation = audiorouter_engine::RuntimeGeneration::new(7);
        bridge
            .scheduler_mut()
            .publish(audiorouter_engine::RuntimeGraph::prepare(
                generation,
                vec![],
            ));
        let input = bridge.scheduler().acquire_input().unwrap();
        bridge.scheduler().submit_input(input).unwrap();
        bridge.scheduler().process_once().unwrap();
        assert_eq!(bridge.scheduler().output().ready(), 1);

        assert_eq!(bridge.reset_stream().unwrap(), 1);
        assert_eq!(bridge.scheduler().output().ready(), 0);
        assert_eq!(
            bridge.scheduler().telemetry().active_generation,
            Some(generation)
        );
        assert_eq!(bridge.timeline_frame(), 0);
    }

    #[test]
    fn quantum_deadline_schedule_advances_with_the_bridge_timeline() {
        let first = std::time::Instant::now();
        let schedule = DeadlineSchedule {
            timeline_frame: 10,
            first_deadline: first,
            quantum_duration: std::time::Duration::from_millis(3),
            quantum_frames: 128,
        };
        assert_eq!(schedule.deadline_for(10), first);
        assert_eq!(
            schedule.deadline_for(138),
            first + std::time::Duration::from_millis(3)
        );
        assert_eq!(
            schedule.deadline_for(266),
            first + std::time::Duration::from_millis(6)
        );
        assert_eq!(schedule.deadline_for(137), first);
    }

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
            channel_mask: 0,
            subformat_guid: String::new(),
        };
        assert_eq!(info.direction, EndpointDirection::Capture);
        assert!(info.minimum_period_100ns <= info.default_period_100ns);
        assert_eq!(info.bytes_per_frame().unwrap(), 8);
    }

    #[test]
    fn endpoint_frame_stride_rejects_invalid_sample_shapes() {
        let mut info = EndpointInfo {
            id: "endpoint".into(),
            direction: EndpointDirection::Render,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 24,
            format_tag: 1,
            channel_mask: 0,
            subformat_guid: String::new(),
        };
        assert_eq!(info.bytes_per_frame().unwrap(), 6);
        info.bits_per_sample = 20;
        assert!(matches!(
            info.bytes_per_frame(),
            Err(AudioError::InvalidFrameSize)
        ));
        info.bits_per_sample = 32;
        info.channels = 0;
        assert!(matches!(
            info.bytes_per_frame(),
            Err(AudioError::InvalidFrameSize)
        ));
    }

    #[test]
    fn endpoint_format_predicate_rejects_integer_32_bit_audio() {
        let mut info = EndpointInfo {
            id: "endpoint".into(),
            direction: EndpointDirection::Capture,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 1,
            channel_mask: 0,
            subformat_guid: String::new(),
        };
        assert!(!info.is_ieee_float32());
        info.format_tag = 0xfffe;
        info.subformat_guid = "{00000003-0000-0010-8000-00aa00389b71}".into();
        assert!(info.is_ieee_float32());
    }

    #[test]
    fn scheduler_bridge_rejects_mismatched_endpoint_formats_before_activation() {
        let endpoint = |direction| EndpointInfo {
            id: match direction {
                EndpointDirection::Capture => "capture",
                EndpointDirection::Render => "render",
            }
            .into(),
            direction,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 3,
            subformat_guid: "{00000003-0000-0010-8000-00AA00389B71}".into(),
        };
        let capture = endpoint(EndpointDirection::Capture);
        let mut render = endpoint(EndpointDirection::Render);
        assert!(WasapiSchedulerBridge::new_for_endpoints(2, &capture, &render, 128, 256).is_ok());
        render.sample_rate_hz = 44_100;
        assert!(matches!(
            WasapiSchedulerBridge::new_for_endpoints(2, &capture, &render, 128, 256),
            Err(AudioError::InvalidFrameSize)
        ));
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
            channel_mask: 0,
            subformat_guid: String::new(),
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
    fn endpoint_snapshot_diff_is_deterministic_when_enumeration_order_changes() {
        let endpoint = |id: &str| EndpointInfo {
            id: id.into(),
            direction: EndpointDirection::Render,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 0,
            subformat_guid: String::new(),
        };
        let before = [endpoint("zulu"), endpoint("alpha")];
        let current = [endpoint("bravo"), endpoint("alpha")];
        let changes = diff_endpoint_snapshots(&before, &current);
        assert_eq!(
            changes,
            vec![
                EndpointChange::Removed(endpoint("zulu")),
                EndpointChange::Added(endpoint("bravo")),
            ]
        );
    }

    #[test]
    fn endpoint_binding_requires_exact_id_and_direction() {
        let endpoint = EndpointInfo {
            id: "stable-id".into(),
            direction: EndpointDirection::Capture,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 1,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 0,
            subformat_guid: String::new(),
        };
        assert_eq!(
            resolve_endpoint_binding(
                std::slice::from_ref(&endpoint),
                "stable-id",
                EndpointDirection::Capture
            ),
            EndpointBindingResolution::Available(endpoint.clone())
        );
        assert_eq!(
            resolve_endpoint_binding(
                std::slice::from_ref(&endpoint),
                "stable-id",
                EndpointDirection::Render
            ),
            EndpointBindingResolution::DirectionChanged {
                id: "stable-id".into(),
                expected: EndpointDirection::Render,
                actual: EndpointDirection::Capture,
            }
        );
        assert_eq!(
            resolve_endpoint_binding(&[endpoint], "gone", EndpointDirection::Capture),
            EndpointBindingResolution::Missing {
                id: "gone".into(),
                direction: EndpointDirection::Capture,
            }
        );
    }

    #[test]
    fn endpoint_binding_rejects_a_changed_mix_format() {
        let expected = EndpointInfo {
            id: "stable-id".into(),
            direction: EndpointDirection::Render,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 0,
            subformat_guid: String::new(),
        };
        let mut actual = expected.clone();
        actual.sample_rate_hz = 44_100;
        assert_eq!(
            resolve_endpoint_binding_with_format(std::slice::from_ref(&actual), &expected),
            EndpointBindingResolution::FormatChanged { expected, actual }
        );
    }

    #[test]
    fn endpoint_binding_rejects_changed_extensible_format_details() {
        let expected = EndpointInfo {
            id: "stable-id".into(),
            direction: EndpointDirection::Capture,
            default_period_100ns: 100_000,
            minimum_period_100ns: 20_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 0xfffe,
            channel_mask: 3,
            subformat_guid: "pcm".into(),
        };
        let mut actual = expected.clone();
        actual.channel_mask = 0x33;
        assert!(matches!(
            resolve_endpoint_binding_with_format(std::slice::from_ref(&actual), &expected),
            EndpointBindingResolution::FormatChanged { .. }
        ));
        actual = expected.clone();
        actual.subformat_guid = "float".into();
        assert!(matches!(
            resolve_endpoint_binding_with_format(std::slice::from_ref(&actual), &expected),
            EndpointBindingResolution::FormatChanged { .. }
        ));
    }

    #[test]
    fn endpoint_binding_error_keeps_structured_resolution() {
        let error = AudioError::EndpointBinding {
            resolution: Box::new(EndpointBindingResolution::Missing {
                id: "gone".into(),
                direction: EndpointDirection::Capture,
            }),
        };
        assert!(matches!(
            error.binding_resolution(),
            Some(EndpointBindingResolution::Missing { id, direction })
                if id == "gone" && *direction == EndpointDirection::Capture
        ));
        assert_eq!(error.kind(), AudioFailureKind::InvalidArgument);
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
        assert!(!error.is_retryable());
        assert_eq!(
            error.remediation(),
            "use a compatible shared-mode endpoint or exclusive stream"
        );
        let busy = AudioError::Windows(windows::core::Error::new(
            windows::core::HRESULT(0x8889000A_u32 as i32),
            "busy",
        ));
        assert!(busy.is_retryable());
        assert_eq!(
            busy.remediation(),
            "identify the owning stream, select another endpoint, or close it and retry"
        );
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
        assert_eq!(
            AudioError::BufferTooSmall {
                required: 8,
                available: 4
            }
            .hresult(),
            0x80070057
        );
        let invalid_argument = AudioError::Windows(windows::core::Error::new(
            windows::core::HRESULT(0x80070057_u32 as i32),
            "invalid argument",
        ));
        assert_eq!(invalid_argument.kind(), AudioFailureKind::InvalidArgument);
        assert!(invalid_argument.to_string().contains("0x80070057"));
        let initialize_failure = AudioError::WindowsOperation {
            operation: "IAudioClient::Initialize(capture)",
            error: windows::core::Error::new(
                windows::core::HRESULT(0x80070057_u32 as i32),
                "invalid argument",
            ),
        };
        assert_eq!(initialize_failure.kind(), AudioFailureKind::InvalidArgument);
        assert_eq!(initialize_failure.hresult(), 0x80070057);
        assert!(initialize_failure
            .to_string()
            .contains("IAudioClient::Initialize(capture)"));
        assert!(AudioFailureKind::DeviceInUse.is_retryable());
        assert!(AudioFailureKind::DeviceInvalidated.is_retryable());
        assert!(AudioFailureKind::ServiceUnavailable.is_retryable());
        assert!(!AudioFailureKind::AccessDenied.is_retryable());
        assert!(!AudioFailureKind::UnsupportedFormat.is_retryable());
        assert_eq!(
            AudioFailureKind::DeviceInUse.remediation(),
            "identify the owning stream, select another endpoint, or close it and retry"
        );
        assert_eq!(
            AudioError::InvalidFrameSize.remediation(),
            "correct the endpoint format or stream request"
        );
    }

    #[test]
    fn capture_initialization_fallback_is_limited_to_exact_e_invalidarg() {
        let invalid_argument = windows::core::Error::new(
            windows::core::HRESULT(0x80070057_u32 as i32),
            "invalid argument",
        );
        let device_in_use = windows::core::Error::new(
            windows::core::HRESULT(0x8889000A_u32 as i32),
            "device in use",
        );
        let access_denied = windows::core::Error::new(
            windows::core::HRESULT(0x80070005_u32 as i32),
            "access denied",
        );

        assert!(should_retry_capture_initialization(&invalid_argument));
        assert!(!should_retry_capture_initialization(&device_in_use));
        assert!(!should_retry_capture_initialization(&access_denied));
    }

    #[test]
    fn audio_failure_codes_are_stable_and_distinguish_contention() {
        assert_eq!(AudioFailureKind::InvalidArgument.code(), "invalidArgument");
        assert_eq!(AudioFailureKind::DeviceInUse.code(), "deviceInUse");
        assert_ne!(
            AudioFailureKind::InvalidArgument.code(),
            AudioFailureKind::DeviceInUse.code()
        );
    }

    #[test]
    fn process_loopback_rejects_zero_target_before_com_activation() {
        for mode in [
            ProcessLoopbackMode::IncludeTargetTree,
            ProcessLoopbackMode::ExcludeTargetTree,
        ] {
            assert!(matches!(
                ProcessLoopbackCapture::open(0, mode),
                Err(AudioError::ApplicationNotFound { process_id: 0 })
            ));
        }
    }

    #[test]
    fn process_loopback_packet_period_policy_is_bounded() {
        assert!(validate_process_loopback_packet_frames(1).is_ok());
        assert!(
            validate_process_loopback_packet_frames(MAX_PROCESS_LOOPBACK_PACKET_FRAMES).is_ok()
        );
        assert!(matches!(
            validate_process_loopback_packet_frames(0),
            Err(AudioError::InvalidFrameSize)
        ));
        assert!(matches!(
            validate_process_loopback_packet_frames(MAX_PROCESS_LOOPBACK_PACKET_FRAMES + 1),
            Err(AudioError::InvalidFrameSize)
        ));
    }

    #[test]
    fn process_loopback_telemetry_has_zero_state_and_saturating_counters() {
        assert_eq!(
            ProcessLoopbackTelemetry::default(),
            ProcessLoopbackTelemetry {
                wait_calls: 0,
                wait_timeouts: 0,
                packets: 0,
                frames: 0,
                minimum_packet_frames: 0,
                maximum_packet_frames: 0,
                silent_packets: 0,
                rejected_packets: 0,
            }
        );

        let counter = std::sync::atomic::AtomicU64::new(u64::MAX);
        saturating_increment(&counter);
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
    }

    #[test]
    fn restart_binding_requires_one_verified_executable_identity() {
        let candidate = ApplicationInfo {
            process_id: 7,
            executable: "Game.EXE".into(),
            executable_path: None,
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

    #[test]
    fn restart_binding_can_require_the_full_executable_path() {
        let applications = [
            ApplicationInfo {
                process_id: 7,
                executable: "Game.EXE".into(),
                executable_path: Some(r"C:\Games\Game.EXE".into()),
                creation_time_100ns: Some(42),
            },
            ApplicationInfo {
                process_id: 8,
                executable: "game.exe".into(),
                executable_path: Some(r"C:\Tools\Game.EXE".into()),
                creation_time_100ns: Some(43),
            },
        ];
        let selected = resolve_application_restart_with_path(
            &applications,
            "game.exe",
            Some(r"c:\games\game.exe"),
        )
        .unwrap();
        assert_eq!(selected.process_id, 7);
        assert!(matches!(
            resolve_application_restart_with_path(
                &applications,
                "game.exe",
                Some(r"C:\Unknown\Game.EXE")
            ),
            Err(AudioError::ApplicationRestartNotFound { .. })
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
        assert!(application.executable_path.is_some());
        let bound = bind_application_with_path(
            process_id,
            &application.executable.to_ascii_lowercase(),
            application.executable_path.as_deref(),
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
    fn restarted_process_cannot_inherit_a_stale_binding() {
        use std::process::Command;
        use std::time::Duration;

        fn launch_helper() -> std::process::Child {
            Command::new("cmd.exe")
                .args(["/C", "ping -n 4 127.0.0.1 > nul"])
                .spawn()
                .expect("launch bounded process identity helper")
        }

        fn observe(child: &std::process::Child) -> ApplicationInfo {
            for _ in 0..100 {
                if let Some(application) = enumerate_applications()
                    .expect("enumerate process identity helper")
                    .into_iter()
                    .find(|application| application.process_id == child.id())
                {
                    return application;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            panic!("process identity helper was not observable")
        }

        let mut first_child = launch_helper();
        let first = observe(&first_child);
        assert_eq!(first.executable.to_ascii_lowercase(), "cmd.exe");
        let first_bound = bind_application(
            first.process_id,
            &first.executable,
            first.creation_time_100ns,
        )
        .expect("fresh process identity should bind");
        assert_eq!(first_bound, first);
        first_child.kill().expect("stop first helper");
        first_child.wait().expect("reap first helper");

        let stale = bind_application(
            first.process_id,
            &first.executable,
            first.creation_time_100ns,
        );
        assert!(matches!(
            stale,
            Err(AudioError::ApplicationNotFound { .. })
                | Err(AudioError::ApplicationIdentityChanged { .. })
        ));

        let mut second_child = launch_helper();
        let second = observe(&second_child);
        assert!(second.creation_time_100ns.is_some());
        if second.process_id == first.process_id {
            assert_ne!(second.creation_time_100ns, first.creation_time_100ns);
            assert!(matches!(
                bind_application(
                    first.process_id,
                    &first.executable,
                    first.creation_time_100ns,
                ),
                Err(AudioError::ApplicationIdentityChanged { .. })
            ));
        }
        second_child.kill().expect("stop second helper");
        second_child.wait().expect("reap second helper");
    }

    #[cfg(windows)]
    #[test]
    fn application_audio_inventory_is_read_only() {
        let inventory = enumerate_application_audio().unwrap();
        assert!(inventory.iter().all(|item| {
            item.process_id != 0
                && item.active_session_count <= item.total_session_count
                && item.capture_session_count <= item.total_session_count
                && item.render_session_count <= item.total_session_count
        }));
    }

    #[cfg(windows)]
    #[test]
    fn application_inventory_is_deterministically_ordered() {
        let applications = enumerate_applications().unwrap();
        assert!(applications.windows(2).all(|pair| {
            pair[0]
                .executable
                .to_ascii_lowercase()
                .cmp(&pair[1].executable.to_ascii_lowercase())
                .then_with(|| pair[0].process_id.cmp(&pair[1].process_id))
                .is_le()
        }));
    }

    #[test]
    fn application_inventory_sort_is_case_insensitive_then_pid() {
        let mut applications = vec![
            ApplicationInfo {
                process_id: 20,
                executable: "zeta.exe".into(),
                executable_path: None,
                creation_time_100ns: Some(2),
            },
            ApplicationInfo {
                process_id: 4,
                executable: "Audio.exe".into(),
                executable_path: None,
                creation_time_100ns: Some(1),
            },
            ApplicationInfo {
                process_id: 3,
                executable: "audio.exe".into(),
                executable_path: None,
                creation_time_100ns: Some(0),
            },
        ];
        sort_application_inventory(&mut applications);
        assert_eq!(
            applications
                .iter()
                .map(|application| (application.executable.as_str(), application.process_id))
                .collect::<Vec<_>>(),
            [("audio.exe", 3), ("Audio.exe", 4), ("zeta.exe", 20)]
        );
    }

    #[test]
    fn application_audio_inventory_sort_is_deterministic_and_deduplicated() {
        let mut inventory = vec![
            ApplicationAudioInfo {
                process_id: 20,
                active_session_count: 1,
                total_session_count: 1,
                capture_session_count: 0,
                render_session_count: 1,
                display_names: vec!["zulu".into(), "alpha".into(), "alpha".into()],
            },
            ApplicationAudioInfo {
                process_id: 4,
                active_session_count: 1,
                total_session_count: 2,
                capture_session_count: 1,
                render_session_count: 1,
                display_names: vec!["voice".into()],
            },
        ];
        sort_application_audio_inventory(&mut inventory);
        assert_eq!(
            inventory
                .iter()
                .map(|item| item.process_id)
                .collect::<Vec<_>>(),
            vec![4, 20]
        );
        assert_eq!(inventory[1].display_names, vec!["alpha", "zulu"]);
    }

    #[test]
    fn audio_display_names_are_bounded_before_retention() {
        let mut names = Vec::new();
        retain_audio_display_name(&mut names, "valid".into());
        retain_audio_display_name(&mut names, "valid".into());
        retain_audio_display_name(
            &mut names,
            "x".repeat(MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES + 1),
        );
        for index in 1..=MAX_APPLICATION_AUDIO_DISPLAY_NAMES {
            retain_audio_display_name(&mut names, format!("name-{index}"));
        }
        assert_eq!(names.len(), MAX_APPLICATION_AUDIO_DISPLAY_NAMES);
        assert!(names.iter().all(|name| {
            !name.is_empty() && name.len() <= MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES
        }));
    }

    #[test]
    fn bounded_recovery_retries_transient_failures_then_succeeds() {
        let mut calls = 0;
        let value = retry_transient_audio_operation(5, 0, || {
            calls += 1;
            if calls < 3 {
                Err(AudioError::Windows(windows::core::Error::new(
                    windows::core::HRESULT(0x88890004_u32 as i32),
                    "device invalidated",
                )))
            } else {
                Ok(42_u32)
            }
        })
        .unwrap();
        assert_eq!(value, 42);
        assert_eq!(calls, 3);
    }

    #[test]
    fn bounded_recovery_does_not_retry_non_transient_failures() {
        let mut calls = 0;
        let result = retry_transient_audio_operation(5, 0, || {
            calls += 1;
            Err::<(), _>(AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x80070057_u32 as i32),
                "invalid argument",
            )))
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn bounded_recovery_caps_transient_attempts() {
        let mut calls = 0;
        let result = retry_transient_audio_operation(99, 0, || {
            calls += 1;
            Err::<(), _>(AudioError::Windows(windows::core::Error::new(
                windows::core::HRESULT(0x88890010_u32 as i32),
                "service unavailable",
            )))
        });
        assert!(result.is_err());
        assert_eq!(calls, 5);
    }

    #[cfg(windows)]
    #[test]
    fn opening_an_unknown_capture_endpoint_fails_without_starting_audio() {
        let error = SharedCapture::open("audiorouter-missing-endpoint", 1_000_000);
        assert!(matches!(error, Err(AudioError::Windows(_))));
    }

    #[test]
    fn capture_retry_is_limited_to_e_invalidarg() {
        let invalid_argument = windows::core::Error::new(
            windows::core::HRESULT(0x80070057u32 as i32),
            "invalid argument",
        );
        let device_in_use = windows::core::Error::new(
            windows::core::HRESULT(0x8889000Au32 as i32),
            "device in use",
        );
        let other_failure = windows::core::Error::new(
            windows::core::HRESULT(0x80004005u32 as i32),
            "unspecified failure",
        );

        assert!(should_retry_capture_initialization(&invalid_argument));
        assert!(!should_retry_capture_initialization(&device_in_use));
        assert!(!should_retry_capture_initialization(&other_failure));
    }

    #[test]
    fn capture_initialize_errors_identify_delivery_mode() {
        assert_eq!(
            capture_initialize_operation(true),
            "IAudioClient::Initialize(capture,event-callback)"
        );
        assert_eq!(
            capture_initialize_operation(false),
            "IAudioClient::Initialize(capture,polling)"
        );
    }

    #[test]
    fn native_bridge_region_round_trips_bounded_blocks_without_audio_access() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-bridge-{}-{}.slot",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let writer = NativeBridgeRegion::create(&path, 2, 128).unwrap();
        let reader = NativeBridgeRegion::open(&path, 2, 128).unwrap();
        let input: Vec<f32> = (0..8).map(|value| value as f32 / 8.0).collect();
        writer.write(4, 1, &input).unwrap();
        let mut output = vec![0.0; input.len()];
        let header = reader.read_into(4, &mut output).unwrap();
        assert_eq!(header.frames, 4);
        assert_eq!(output, input);
        assert!(matches!(
            reader.read_into(5, &mut output),
            Err(NativeBridgeRegionError::StaleGeneration)
        ));
        assert!(matches!(
            reader.read_into_after(4, 1, &mut output),
            Err(NativeBridgeRegionError::SequenceRegression)
        ));
        assert!(matches!(
            writer.write(4, 1, &input),
            Err(NativeBridgeRegionError::SequenceRegression)
        ));
        drop(reader);
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn native_bridge_realtime_writer_publishes_engine_tap_without_allocation() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-tap-{}-{}.slot",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let region = NativeBridgeRegion::create(&path, 2, 128).unwrap();
        let reader = NativeBridgeRegion::open(&path, 2, 128).unwrap();
        let writer = NativeBridgeRealtimeWriter::new(region, 9).unwrap();
        let mut block = audiorouter_engine::AudioBlock::new(2, 4).unwrap();
        block
            .copy_from_interleaved(&[1.0, 10.0, 2.0, 20.0, 3.0, 30.0, 4.0, 40.0])
            .unwrap();
        <NativeBridgeRealtimeWriter as audiorouter_engine::AudioTap>::on_processed_block(
            &writer, 0, &block,
        );
        assert_eq!(writer.published_blocks(), 1);
        let mut output = [0.0; 8];
        reader.read_into(9, &mut output).unwrap();
        assert_eq!(output, [1.0, 10.0, 2.0, 20.0, 3.0, 30.0, 4.0, 40.0]);
        drop(reader);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn native_bridge_region_rejects_invalid_shapes_before_mapping() {
        let path =
            std::env::temp_dir().join(format!("audiorouter-invalid-bridge-{}", std::process::id()));
        assert!(matches!(
            NativeBridgeRegion::create(&path, 3, 128),
            Err(NativeBridgeRegionError::InvalidFrame)
        ));
        assert!(matches!(
            NativeBridgeRegion::create(&path, 2, 0),
            Err(NativeBridgeRegionError::InvalidFrame)
        ));
    }

    #[test]
    fn native_bridge_session_negotiates_identity_and_assigns_sequences() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-session-{}-{}.slot",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-main".to_owned(),
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            generation: 9,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 4,
            lease_ms: 1_000,
        };
        let mut writer = NativeBridgeSession::create(&path, hello.clone()).unwrap();
        let mut reader_hello = hello;
        reader_hello.direction = audiorouter_protocol::AudioBridgeDirection::RenderSource;
        let reader = NativeBridgeSession::open(&path, reader_hello).unwrap();
        let input = [0.25_f32, -0.25, 0.5, -0.5];
        assert_eq!(writer.write(&input).unwrap(), 1);
        assert_eq!(writer.write(&input).unwrap(), 2);
        let mut output = [0.0; 4];
        let header = reader.read_into(&mut output).unwrap();
        assert_eq!(header.sequence, 2);
        assert!(matches!(
            reader.read_into_after(2, &mut output),
            Err(NativeBridgeSessionError::Region(
                NativeBridgeRegionError::SequenceRegression
            ))
        ));
        assert_eq!(output, input);
        assert_eq!(reader.hello().bus_id, "bus-main");
        drop(reader);
        drop(writer);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn native_bridge_session_exposes_a_second_realtime_producer_view() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-session-tap-{}-{}.slot",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-tap".to_owned(),
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            generation: 11,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 4,
            lease_ms: 1_000,
        };
        let session = NativeBridgeSession::create(&path, hello).unwrap();
        let tap = session.realtime_writer().unwrap();
        let reader = NativeBridgeRegion::open(&path, 2, 4).unwrap();
        let mut block = audiorouter_engine::AudioBlock::new(2, 2).unwrap();
        block.copy_from_interleaved(&[0.1, 0.2, 0.3, 0.4]).unwrap();
        <NativeBridgeRealtimeWriter as audiorouter_engine::AudioTap>::on_processed_block(
            &tap, 0, &block,
        );
        let mut output = [0.0; 4];
        reader.read_into(11, &mut output).unwrap();
        assert_eq!(output, [0.1, 0.2, 0.3, 0.4]);
        drop(reader);
        drop(tap);
        drop(session);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn native_bridge_session_enforces_directional_read_write_roles() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-direction-{}-{}.slot",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-direction".into(),
            direction: audiorouter_protocol::AudioBridgeDirection::RenderSource,
            generation: 12,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 4,
            lease_ms: 1_000,
        };
        let mut session = NativeBridgeSession::create(&path, hello).unwrap();
        assert!(matches!(
            session.write(&[0.0, 0.0]),
            Err(NativeBridgeSessionError::WrongDirection)
        ));
        assert!(matches!(
            session.realtime_writer(),
            Err(NativeBridgeSessionError::WrongDirection)
        ));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn native_bridge_session_rejects_invalid_identity_before_file_creation() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-invalid-session-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: String::new(),
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            generation: 1,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        assert!(matches!(
            NativeBridgeSession::create(&path, hello),
            Err(NativeBridgeSessionError::InvalidHello(
                audiorouter_protocol::AudioBridgeContractError::EmptyBusId
            ))
        ));
        assert!(!path.exists());
    }

    #[test]
    fn native_bridge_session_expires_and_cannot_be_revived_by_a_late_heartbeat() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-expired-session-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-main".to_owned(),
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            generation: 10,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 4,
            lease_ms: 1_000,
        };
        let mut session = NativeBridgeSession::create(&path, hello).unwrap();
        let base = std::time::Instant::now();
        session.heartbeat_at(base).unwrap();
        let expired = base + std::time::Duration::from_millis(1_001);
        assert!(session.is_lease_expired_at(expired));
        assert!(matches!(
            session.heartbeat_at(expired),
            Err(NativeBridgeSessionError::LeaseExpired)
        ));
        assert!(matches!(
            session.write_at(expired, &[0.0; 4]),
            Err(NativeBridgeSessionError::LeaseExpired)
        ));
        drop(session);
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_bridge_control_request_matches_bounded_driver_layout() {
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-main".to_owned(),
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            generation: 11,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        let request = native_bridge_open_request(&hello, 0, 0).unwrap();
        assert_eq!(std::mem::size_of::<NativeBridgeOpenRequest>(), 176);
        assert_eq!(request.bus_id_bytes, 16);
        assert_eq!(
            &request.bus_id[..8],
            "bus-main".encode_utf16().collect::<Vec<_>>()
        );
        assert_eq!(request.generation, 11);
        assert_eq!(request.direction, 2);
        let mut render_hello = hello.clone();
        render_hello.direction = audiorouter_protocol::AudioBridgeDirection::RenderSource;
        assert_eq!(
            native_bridge_open_request(&render_hello, 0, 0)
                .unwrap()
                .direction,
            1
        );
        assert!(native_bridge_open_request(
            &audiorouter_protocol::AudioBridgeHello {
                bus_id: "x".repeat(65),
                ..hello.clone()
            },
            0,
            0,
        )
        .is_err());
        assert!(native_bridge_open_request(&hello, 1, 31).is_err());
        assert!(native_bridge_open_request(&hello, 1, 32).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn native_bridge_duplex_rejects_mismatched_directions_before_device_open() {
        let render = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-duplex".into(),
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            generation: 1,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        let capture = render.clone();
        assert!(matches!(
            NativeBridgeDuplexController::create(
                "\\\\.\\AudioRouterVirtualBridge",
                "C:\\missing-render.slot",
                "C:\\missing-capture.slot",
                render,
                capture,
            ),
            Err(NativeBridgeControllerError::InvalidDuplex)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn native_bridge_duplex_rejects_mismatched_bus_before_device_open() {
        let render = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "bus-render".into(),
            direction: audiorouter_protocol::AudioBridgeDirection::RenderSource,
            generation: 1,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        let mut capture = render.clone();
        capture.bus_id = "bus-capture".into();
        capture.direction = audiorouter_protocol::AudioBridgeDirection::CaptureSink;
        assert!(matches!(
            NativeBridgeDuplexController::create(
                "\\\\.\\AudioRouterVirtualBridge",
                "C:\\missing-render.slot",
                "C:\\missing-capture.slot",
                render,
                capture,
            ),
            Err(NativeBridgeControllerError::InvalidDuplex)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn native_bridge_section_handle_uses_only_the_temporary_bridge_file() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-section-{}-{}.slot",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let region = NativeBridgeRegion::create(&path, 2, 128).unwrap();
        let mapping_bytes = region.mapping_bytes();
        let section = NativeBridgeSectionHandle::for_file(&path, mapping_bytes as u32).unwrap();
        assert_ne!(section.raw_handle(), 0);
        assert_eq!(section.mapping_bytes(), mapping_bytes as u32);
        region.write(12, 1, &[0.25, -0.25, 0.5, -0.5]).unwrap();
        let generation = section.read_bytes(8, 8).unwrap();
        assert_eq!(generation, 12_u64.to_le_bytes());
        drop(section);
        drop(region);
        std::fs::remove_file(path).unwrap();
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
