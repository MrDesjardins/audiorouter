//! Windows named-pipe transport for the local JSON-RPC boundary.
//!
//! The transport deliberately does not change audio configuration or open an
//! audio endpoint.  Authentication/ACL policy is a separate boundary and is
//! not implied by the default pipe security descriptor used by this prototype.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    UnsupportedPlatform,
    InvalidPipeName,
    Windows(String),
    Protocol(String),
    UnexpectedEof,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TransportError {}

#[cfg(windows)]
mod windows_pipe {
    use super::TransportError;
    use audiorouter_protocol::{decode_frame, encode_frame, MAX_FRAME_BYTES};
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, LocalFree, ERROR_ALREADY_EXISTS, GENERIC_READ, GENERIC_WRITE,
        HANDLE, HLOCAL, INVALID_HANDLE_VALUE,
    };
    use windows::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
    };
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_SHARE_NONE, OPEN_EXISTING,
        PIPE_ACCESS_DUPLEX,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
        PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    };
    use windows::Win32::System::Threading::{
        CreateMutexW, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(once(0)).collect()
    }

    fn singleton_name(pipe_name: &str, user_sid: &str) -> Vec<u16> {
        let suffix: String = format!("{user_sid}-{pipe_name}")
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        wide(&format!(r"Local\AudioRouter-{suffix}"))
    }

    fn acquire_singleton(pipe_name: &str) -> Result<Handle, TransportError> {
        let name = singleton_name(pipe_name, &current_user_sid()?);
        let handle =
            unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }.map_err(win_error)?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            drop(Handle(handle));
            return Err(TransportError::Windows(
                "another backend already owns this user pipe".into(),
            ));
        }
        Ok(Handle(handle))
    }

    #[must_use = "the singleton handle must stay alive while serving"]
    pub struct ServerSingleton(Handle);

    impl Drop for ServerSingleton {
        fn drop(&mut self) {
            let _ = &self.0;
        }
    }

    pub fn acquire_server_singleton(pipe_name: &str) -> Result<ServerSingleton, TransportError> {
        Ok(ServerSingleton(acquire_singleton(pipe_name)?))
    }

    fn check_name(name: &str) -> Result<(), TransportError> {
        if !name.starts_with(r"\\.\pipe\") || name.len() <= 9 || name.contains('\0') {
            Err(TransportError::InvalidPipeName)
        } else {
            Ok(())
        }
    }

    fn win_error(error: impl std::fmt::Display) -> TransportError {
        TransportError::Windows(error.to_string())
    }

    fn token_user_sid_string(token: HANDLE) -> Result<String, TransportError> {
        let mut required = 0;
        let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
        if required == 0 {
            return Err(TransportError::Windows(
                "token user information size was zero".into(),
            ));
        }
        let mut buffer = vec![0u8; required as usize];
        unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                buffer.len() as u32,
                &mut required,
            )
        }
        .map_err(win_error)?;
        let user = unsafe { &*(buffer.as_ptr().cast::<TOKEN_USER>()) };
        let mut string_sid = windows::core::PWSTR::null();
        unsafe { ConvertSidToStringSidW(user.User.Sid, &mut string_sid) }.map_err(win_error)?;
        if string_sid.is_null() {
            return Err(TransportError::Windows(
                "Windows returned a null SID string".into(),
            ));
        }
        let text = unsafe {
            let mut length = 0;
            while *string_sid.0.add(length) != 0 {
                length += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(string_sid.0, length))
        };
        unsafe { LocalFree(Some(HLOCAL(string_sid.0.cast()))) };
        Ok(text)
    }

    pub fn client_user_sid(client_process_id: u32) -> Result<String, TransportError> {
        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, client_process_id) }
                .map_err(win_error)?;
        let process = Handle(process);
        let mut token = INVALID_HANDLE_VALUE;
        unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) }.map_err(win_error)?;
        let token = Handle(token);
        token_user_sid_string(token.0)
    }

    pub fn current_user_sid() -> Result<String, TransportError> {
        client_user_sid(std::process::id())
    }

    struct SecurityDescriptor(windows::Win32::Security::PSECURITY_DESCRIPTOR);
    impl Drop for SecurityDescriptor {
        fn drop(&mut self) {
            if !self.0 .0.is_null() {
                unsafe { LocalFree(Some(HLOCAL(self.0 .0))) };
            }
        }
    }

    fn owner_only_security() -> Result<SecurityDescriptor, TransportError> {
        let mut descriptor = windows::Win32::Security::PSECURITY_DESCRIPTOR(std::ptr::null_mut());
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                windows::core::w!("D:P(A;;GA;;;OW)"),
                1,
                std::ptr::addr_of_mut!(descriptor),
                None,
            )
        }
        .map_err(win_error)?;
        Ok(SecurityDescriptor(descriptor))
    }

    /// Compare the connected client process token's user SID with this process.
    /// This is a same-user check, not a replacement for a restrictive pipe ACL.
    pub fn client_is_same_user(client_process_id: u32) -> Result<bool, TransportError> {
        let process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, client_process_id) }
                .map_err(win_error)?;
        let process = Handle(process);
        let mut client_token = INVALID_HANDLE_VALUE;
        unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut client_token) }
            .map_err(win_error)?;
        let client_token = Handle(client_token);

        let current_process =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, std::process::id()) }
                .map_err(win_error)?;
        let current_process = Handle(current_process);
        let mut current_token = INVALID_HANDLE_VALUE;
        unsafe { OpenProcessToken(current_process.0, TOKEN_QUERY, &mut current_token) }
            .map_err(win_error)?;
        let current_token = Handle(current_token);

        Ok(token_user_sid_string(client_token.0)? == token_user_sid_string(current_token.0)?)
    }

    struct Handle(HANDLE);
    unsafe impl Send for Handle {}
    impl Drop for Handle {
        fn drop(&mut self) {
            if self.0 != INVALID_HANDLE_VALUE && !self.0.is_invalid() {
                let _ = unsafe { CloseHandle(self.0) };
            }
        }
    }

    fn read_exact(handle: HANDLE, mut output: &mut [u8]) -> Result<(), TransportError> {
        while !output.is_empty() {
            let mut count = 0;
            unsafe { ReadFile(handle, Some(output), Some(&mut count), None) }.map_err(win_error)?;
            if count == 0 {
                return Err(TransportError::UnexpectedEof);
            }
            output = &mut output[count as usize..];
        }
        Ok(())
    }

    fn write_all(handle: HANDLE, mut input: &[u8]) -> Result<(), TransportError> {
        while !input.is_empty() {
            let mut count = 0;
            unsafe { WriteFile(handle, Some(input), Some(&mut count), None) }.map_err(win_error)?;
            if count == 0 {
                return Err(TransportError::UnexpectedEof);
            }
            input = &input[count as usize..];
        }
        Ok(())
    }

    fn read_frame(handle: HANDLE) -> Result<Vec<u8>, TransportError> {
        let mut header = [0u8; 4];
        read_exact(handle, &mut header)?;
        let length = u32::from_le_bytes(header) as usize;
        if length > MAX_FRAME_BYTES {
            return Err(TransportError::Protocol("frame exceeds maximum".into()));
        }
        let mut frame = Vec::with_capacity(4 + length);
        frame.extend_from_slice(&header);
        frame.resize(4 + length, 0);
        read_exact(handle, &mut frame[4..])?;
        Ok(frame)
    }

    fn accept_client(name: &str) -> Result<(Handle, u32), TransportError> {
        check_name(name)?;
        let name = wide(name);
        let security = owner_only_security()?;
        let attributes = windows::Win32::Security::SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<windows::Win32::Security::SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.0 .0,
            bInheritHandle: false.into(),
        };
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                (MAX_FRAME_BYTES + 4) as u32,
                (MAX_FRAME_BYTES + 4) as u32,
                0,
                Some(&attributes),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_invalid() {
            return Err(win_error(windows::core::Error::from_thread()));
        }
        let handle = Handle(handle);
        if let Err(error) = unsafe { ConnectNamedPipe(handle.0, None) } {
            if error.code().0 != 0x8007_0217u32 as i32 {
                return Err(win_error(error));
            }
        }
        let mut client_process_id = 0;
        unsafe { GetNamedPipeClientProcessId(handle.0, &mut client_process_id) }
            .map_err(win_error)?;
        if client_process_id == 0 {
            return Err(TransportError::Windows(
                "named pipe returned no client process ID".into(),
            ));
        }
        if !client_is_same_user(client_process_id)? {
            return Err(TransportError::Windows(
                "named pipe client is not the server user".into(),
            ));
        }
        Ok((handle, client_process_id))
    }

    /// Serve exactly one framed request, then disconnect and close the pipe.
    /// The pipe is created with an owner-only ACL and the client SID is checked
    /// before its request is read. Production callers should still review the
    /// deployment account/service model and authorization scopes.
    pub fn serve_once<F>(name: &str, handler: F) -> Result<(), TransportError>
    where
        F: FnOnce(&[u8]) -> Result<Vec<u8>, TransportError>,
    {
        serve_once_with_client(name, |_, frame| handler(frame))
    }

    /// Serve one request and provide the connected client's Windows process ID.
    /// The process ID is an identity input, not authentication by itself; callers
    /// must still validate the process token/SID before allowing sensitive methods.
    pub fn serve_once_with_client<F>(name: &str, handler: F) -> Result<(), TransportError>
    where
        F: FnOnce(u32, &[u8]) -> Result<Vec<u8>, TransportError>,
    {
        serve_once_with_client_optional(name, |client_pid, frame| {
            handler(client_pid, frame).map(Some)
        })
    }

    /// Serve one request where `None` means JSON-RPC notification/no response.
    pub fn serve_once_with_client_optional<F>(name: &str, handler: F) -> Result<(), TransportError>
    where
        F: FnOnce(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
    {
        let (handle, client_process_id) = accept_client(name)?;
        let request = read_frame(handle.0)?;
        if let Some(response) = handler(client_process_id, &request)? {
            write_all(handle.0, &response)?;
            unsafe { FlushFileBuffers(handle.0) }.map_err(win_error)?;
        }
        let _ = unsafe { DisconnectNamedPipe(handle.0) };
        Ok(())
    }

    /// Serve a bounded persistent client session. The same authenticated pipe
    /// connection may carry `frames` requests; it is disconnected afterward so
    /// ownership and shutdown remain deterministic for callers and tests.
    pub fn serve_session<F>(name: &str, frames: usize, mut handler: F) -> Result<(), TransportError>
    where
        F: FnMut(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
    {
        if frames == 0 {
            return Err(TransportError::Protocol(
                "session must serve at least one frame".into(),
            ));
        }
        let (handle, client_process_id) = accept_client(name)?;
        for _ in 0..frames {
            let request = read_frame(handle.0)?;
            if let Some(response) = handler(client_process_id, &request)? {
                write_all(handle.0, &response)?;
                unsafe { FlushFileBuffers(handle.0) }.map_err(win_error)?;
            }
        }
        let _ = unsafe { DisconnectNamedPipe(handle.0) };
        Ok(())
    }

    /// Serve a fixed number of sequential authenticated connections.
    /// A bounded loop makes lifecycle tests deterministic; a production daemon
    /// can own the outer restart/shutdown policy around `serve_once_with_client`.
    pub fn serve_connections<F>(
        name: &str,
        connections: usize,
        mut handler: F,
    ) -> Result<(), TransportError>
    where
        F: FnMut(u32, &[u8]) -> Result<Vec<u8>, TransportError>,
    {
        let _singleton = acquire_singleton(name)?;
        for _ in 0..connections {
            serve_once_with_client(name, |client_pid, frame| handler(client_pid, frame))?;
        }
        Ok(())
    }

    /// Connect to a local named pipe and exchange one framed message.
    pub fn round_trip(name: &str, request: &[u8]) -> Result<Vec<u8>, TransportError> {
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = (0..20)
            .find_map(|_| {
                let result = unsafe {
                    CreateFileW(
                        PCWSTR(name.as_ptr()),
                        (GENERIC_READ | GENERIC_WRITE).0,
                        FILE_SHARE_NONE,
                        None,
                        OPEN_EXISTING,
                        Default::default(),
                        None,
                    )
                };
                match result {
                    Ok(handle) => Some(Ok(handle)),
                    Err(error)
                        if matches!(
                            error.code().0,
                            x if x == 0x8007_00E7u32 as i32 || x == 0x8007_0002u32 as i32
                        ) =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        None
                    }
                    Err(error) => Some(Err(win_error(error))),
                }
            })
            .unwrap_or_else(|| {
                Err(TransportError::Windows(
                    "timed out waiting for a free named-pipe instance".into(),
                ))
            })?;
        let handle = Handle(handle);
        write_all(handle.0, request)?;
        read_frame(handle.0)
    }

    /// Exchange one request frame and read exactly `responses` response frames.
    pub fn round_trip_many(
        name: &str,
        request: &[u8],
        responses: usize,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = (0..20)
            .find_map(|_| {
                let result = unsafe {
                    CreateFileW(
                        PCWSTR(name.as_ptr()),
                        (GENERIC_READ | GENERIC_WRITE).0,
                        FILE_SHARE_NONE,
                        None,
                        OPEN_EXISTING,
                        Default::default(),
                        None,
                    )
                };
                match result {
                    Ok(handle) => Some(Ok(handle)),
                    Err(error)
                        if matches!(
                            error.code().0,
                            x if x == 0x8007_00E7u32 as i32 || x == 0x8007_0002u32 as i32
                        ) =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        None
                    }
                    Err(error) => Some(Err(win_error(error))),
                }
            })
            .unwrap_or_else(|| {
                Err(TransportError::Windows(
                    "timed out waiting for a free named-pipe instance".into(),
                ))
            })?;
        let handle = Handle(handle);
        write_all(handle.0, request)?;
        (0..responses).map(|_| read_frame(handle.0)).collect()
    }

    /// Exchange the same framed request repeatedly over one authenticated
    /// connection. This is a deterministic transport primitive for exercising
    /// subscription/reconnect lifetimes; callers can encode distinct frames
    /// with `round_trip_many` or a higher-level client protocol.
    pub fn round_trip_session(
        name: &str,
        request: &[u8],
        frames: usize,
    ) -> Result<Vec<Vec<u8>>, TransportError> {
        if frames == 0 {
            return Err(TransportError::Protocol(
                "session must exchange at least one frame".into(),
            ));
        }
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = (0..20)
            .find_map(|_| {
                let result = unsafe {
                    CreateFileW(
                        PCWSTR(name.as_ptr()),
                        (GENERIC_READ | GENERIC_WRITE).0,
                        FILE_SHARE_NONE,
                        None,
                        OPEN_EXISTING,
                        Default::default(),
                        None,
                    )
                };
                match result {
                    Ok(handle) => Some(Ok(handle)),
                    Err(error)
                        if matches!(
                            error.code().0,
                            x if x == 0x8007_00E7u32 as i32 || x == 0x8007_0002u32 as i32
                        ) =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        None
                    }
                    Err(error) => Some(Err(win_error(error))),
                }
            })
            .unwrap_or_else(|| {
                Err(TransportError::Windows(
                    "timed out waiting for a free named-pipe instance".into(),
                ))
            })?;
        let handle = Handle(handle);
        let mut responses = Vec::with_capacity(frames);
        for _ in 0..frames {
            write_all(handle.0, request)?;
            responses.push(read_frame(handle.0)?);
        }
        Ok(responses)
    }

    /// Send a notification frame and close after the server has received it.
    pub fn send_oneway(name: &str, request: &[u8]) -> Result<(), TransportError> {
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = (0..20)
            .find_map(|_| {
                let result = unsafe {
                    CreateFileW(
                        PCWSTR(name.as_ptr()),
                        (GENERIC_READ | GENERIC_WRITE).0,
                        FILE_SHARE_NONE,
                        None,
                        OPEN_EXISTING,
                        Default::default(),
                        None,
                    )
                };
                match result {
                    Ok(handle) => Some(Ok(handle)),
                    Err(error)
                        if matches!(
                            error.code().0,
                            x if x == 0x8007_00E7u32 as i32 || x == 0x8007_0002u32 as i32
                        ) =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        None
                    }
                    Err(error) => Some(Err(win_error(error))),
                }
            })
            .unwrap_or_else(|| {
                Err(TransportError::Windows(
                    "timed out waiting for a free named-pipe instance".into(),
                ))
            })?;
        let handle = Handle(handle);
        write_all(handle.0, request)
    }

    pub fn echo_handler(frame: &[u8]) -> Result<Vec<u8>, TransportError> {
        decode_frame::<serde_json::Value>(frame)
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        encode_frame(&serde_json::json!({"ok": true}))
            .map_err(|e| TransportError::Protocol(e.to_string()))
    }
}

#[cfg(windows)]
pub use windows_pipe::{
    acquire_server_singleton, client_is_same_user, client_user_sid, current_user_sid, echo_handler,
    round_trip, round_trip_many, round_trip_session, send_oneway, serve_connections, serve_once,
    serve_once_with_client, serve_once_with_client_optional, serve_session, ServerSingleton,
};

#[cfg(windows)]
pub fn serve_control_connections(
    name: &str,
    connections: usize,
    mut plane: audiorouter_control::ControlPlane,
    grant: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    let _singleton = acquire_server_singleton(name)?;
    for _ in 0..connections {
        serve_once_with_client_optional(name, |client_pid, frame| {
            let client_id = client_user_sid(client_pid)?;
            let responses = plane
                .dispatch_frame_authorized_for_client(frame, &client_id, &grant)
                .map_err(|error| TransportError::Protocol(error.to_string()))?;
            if responses.is_empty() {
                Ok(None)
            } else {
                let total = responses.iter().map(Vec::len).sum();
                let mut combined = Vec::with_capacity(total);
                for response in responses {
                    combined.extend_from_slice(&response);
                }
                Ok(Some(combined))
            }
        })?;
    }
    Ok(())
}

#[cfg(windows)]
/// Serve a bounded number of authenticated persistent sessions. Each session
/// accepts a fixed number of framed requests before disconnecting, preserving
/// control-plane state across reconnects without introducing an unbounded
/// daemon loop.
pub fn serve_control_sessions(
    name: &str,
    sessions: usize,
    frames_per_session: usize,
    mut plane: audiorouter_control::ControlPlane,
    grant: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    let _singleton = acquire_server_singleton(name)?;
    for _ in 0..sessions {
        serve_session(name, frames_per_session, |client_pid, frame| {
            let client_id = client_user_sid(client_pid)?;
            let responses = plane
                .dispatch_frame_authorized_for_client(frame, &client_id, &grant)
                .map_err(|error| TransportError::Protocol(error.to_string()))?;
            if responses.is_empty() {
                Ok(None)
            } else {
                let total = responses.iter().map(Vec::len).sum();
                let mut combined = Vec::with_capacity(total);
                for response in responses {
                    combined.extend_from_slice(&response);
                }
                Ok(Some(combined))
            }
        })?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn serve_control_connections_as_role(
    name: &str,
    connections: usize,
    plane: audiorouter_control::ControlPlane,
    role: audiorouter_control::ClientRole,
) -> Result<(), TransportError> {
    serve_control_connections(
        name,
        connections,
        plane,
        audiorouter_control::ClientGrant::for_role(role),
    )
}

#[cfg(windows)]
pub fn serve_control_connections_for_current_user(
    name: &str,
    connections: usize,
    plane: audiorouter_control::ControlPlane,
) -> Result<(), TransportError> {
    let sid = current_user_sid()?;
    let grant = plane
        .grant_for_client(&sid)
        .map_err(|error| TransportError::Protocol(format!("enrollment lookup failed: {error:?}")))?
        .ok_or_else(|| TransportError::Windows("current user is not enrolled".into()))?;
    serve_control_connections(name, connections, plane, grant)
}

#[cfg(not(windows))]
pub fn serve_control_sessions(
    _: &str,
    _: usize,
    _: usize,
    _: audiorouter_control::ControlPlane,
    _: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn round_trip(_: &str, _: &[u8]) -> Result<Vec<u8>, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_once<F>(_: &str, _: F) -> Result<(), TransportError>
where
    F: FnOnce(&[u8]) -> Result<Vec<u8>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_once_with_client_optional<F>(_: &str, _: F) -> Result<(), TransportError>
where
    F: FnOnce(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn send_oneway(_: &str, _: &[u8]) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn round_trip_many(_: &str, _: &[u8], _: usize) -> Result<Vec<Vec<u8>>, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn round_trip_session(_: &str, _: &[u8], _: usize) -> Result<Vec<Vec<u8>>, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_session<F>(_: &str, _: usize, _: F) -> Result<(), TransportError>
where
    F: FnMut(u32, &[u8]) -> Result<Option<Vec<u8>>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn client_is_same_user(_: u32) -> Result<bool, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn client_user_sid(_: u32) -> Result<String, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn current_user_sid() -> Result<String, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_control_connections_for_current_user(
    _: &str,
    _: usize,
    _: audiorouter_control::ControlPlane,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_connections<F>(_: &str, _: usize, _: F) -> Result<(), TransportError>
where
    F: FnMut(u32, &[u8]) -> Result<Vec<u8>, TransportError>,
{
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_control_connections(
    _: &str,
    _: usize,
    _: audiorouter_control::ControlPlane,
    _: audiorouter_control::ClientGrant,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(not(windows))]
pub fn serve_control_connections_as_role(
    _: &str,
    _: usize,
    _: audiorouter_control::ControlPlane,
    _: audiorouter_control::ClientRole,
) -> Result<(), TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_control::{ClientGrant, ClientRole, ControlPlane};
    use audiorouter_protocol::encode_frame;

    #[test]
    fn rejects_non_pipe_names_without_touching_the_system() {
        let result = round_trip("not-a-pipe", &[]);
        assert_eq!(result, Err(TransportError::InvalidPipeName));
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_round_trip() {
        let name = format!(r"\\.\pipe\audiorouter-test-{}", std::process::id());
        let request =
            encode_frame(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"system.describe"}))
                .unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || serve_once(&server_name, echo_handler));
        std::thread::sleep(std::time::Duration::from_millis(20));
        let response = match round_trip(&name, &request) {
            Ok(response) => response,
            Err(error) => panic!(
                "client failed: {error:?}; server: {:?}",
                server.join().unwrap()
            ),
        };
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap()["ok"],
            true
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_dispatches_control_plane_json_rpc() {
        let name = format!(r"\\.\pipe\audiorouter-control-test-{}", std::process::id());
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "system.describe"
        }))
        .unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_control_connections_as_role(
                &server_name,
                1,
                ControlPlane::new("native-test"),
                ClientRole::Observer,
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let response = round_trip(&name, &request).unwrap();
        let response = serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap();
        assert_eq!(response["id"], 9);
        assert_eq!(response["result"]["protocolVersion"]["major"], 1);
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_applies_grants_before_mutation() {
        let name = format!(r"\\.\pipe\audiorouter-auth-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            let mut plane = ControlPlane::new("native-auth-test");
            serve_once(&server_name, |frame| {
                let responses = plane
                    .dispatch_frame_authorized(frame, &ClientGrant::read_only())
                    .map_err(|error| TransportError::Protocol(error.to_string()))?;
                responses.into_iter().next().ok_or_else(|| {
                    TransportError::Protocol("missing authorization response".into())
                })
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "graph.commit"
        }))
        .unwrap();
        let response = round_trip(&name, &request).unwrap();
        let response = serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap();
        assert_eq!(response["error"]["code"], -32001);
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_notification_has_no_response_and_does_not_block() {
        let name = format!(
            r"\\.\pipe\audiorouter-notification-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_once_with_client_optional(&server_name, |_, frame| {
                let message = audiorouter_protocol::decode_rpc_frame(frame)
                    .map_err(|error| TransportError::Protocol(error.to_string()))?;
                assert!(
                    matches!(message, audiorouter_protocol::RpcMessage::Single(request) if request.is_notification())
                );
                Ok(None)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "system.describe"
        }))
        .unwrap();
        send_oneway(&name, &request).unwrap();
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_preserves_all_batch_responses() {
        let name = format!(r"\\.\pipe\audiorouter-batch-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_control_connections(
                &server_name,
                1,
                ControlPlane::new("native-batch-test"),
                ClientGrant::read_only(),
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!([
            {"jsonrpc":"2.0","id":21,"method":"system.describe"},
            {"jsonrpc":"2.0","id":22,"method":"graph.commit"}
        ]))
        .unwrap();
        let responses = round_trip_many(&name, &request, 2).unwrap();
        assert_eq!(responses.len(), 2);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[0][4..]).unwrap()["id"],
            21
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap()["id"],
            22
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap()["error"]
                ["code"],
            -32001
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_rejects_oversized_frame_before_dispatch() {
        let name = format!(
            r"\\.\pipe\audiorouter-oversized-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_control_connections(
                &server_name,
                1,
                ControlPlane::new("native-oversized-test"),
                ClientGrant::read_only(),
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = ((audiorouter_protocol::MAX_FRAME_BYTES as u32) + 1)
            .to_le_bytes()
            .to_vec();
        let result = round_trip(&name, &request);
        assert!(result.is_err());
        let server_result = server.join().unwrap();
        assert!(matches!(server_result, Err(TransportError::Protocol(_))));
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_exposes_connected_client_process_id() {
        let name = format!(r"\\.\pipe\audiorouter-peer-test-{}", std::process::id());
        let request =
            encode_frame(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"status.get"}))
                .unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_once_with_client(&server_name, |client_pid, frame| {
                assert_eq!(client_pid, std::process::id());
                assert!(client_is_same_user(client_pid).unwrap());
                echo_handler(frame)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        round_trip(&name, &request).unwrap();
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_binds_current_user_enrollment_to_authenticated_sid() {
        let name = format!(r"\\.\pipe\audiorouter-enrolled-test-{}", std::process::id());
        let sid = current_user_sid().unwrap();
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            let mut plane = ControlPlane::new("native-enrolled-test");
            plane.enroll_client(sid, ClientRole::Observer).unwrap();
            serve_control_connections_for_current_user(&server_name, 1, plane)
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "system.describe"
        }))
        .unwrap();
        let response = round_trip(&name, &request).unwrap();
        let response = serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap();
        assert_eq!(response["id"], 31);
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn authenticated_server_accepts_sequential_connections() {
        let name = format!(r"\\.\pipe\audiorouter-loop-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, 2, |client_pid, frame| {
                assert!(client_is_same_user(client_pid).unwrap());
                echo_handler(frame)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        for id in [1, 2] {
            let request = encode_frame(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "status.get"
            }))
            .unwrap();
            round_trip(&name, &request).unwrap();
        }
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_keeps_one_authenticated_session_for_bounded_frames() {
        let name = format!(r"\\.\pipe\audiorouter-session-test-{}", std::process::id());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_session(&server_name, 2, |client_pid, frame| {
                assert!(client_is_same_user(client_pid).unwrap());
                echo_handler(frame).map(Some)
            })
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 77,
            "method": "status.get"
        }))
        .unwrap();
        let responses = round_trip_session(&name, &request, 2).unwrap();
        assert_eq!(responses.len(), 2);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&responses[1][4..]).unwrap()["ok"],
            true
        );
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_backend_is_singleton_per_user_name() {
        let name = format!(
            r"\\.\pipe\audiorouter-singleton-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, 2, |_, frame| echo_handler(frame))
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let competing = serve_connections(&name, 1, |_, frame| echo_handler(frame));
        assert!(matches!(
            competing,
            Err(TransportError::Windows(message)) if message.contains("already owns")
        ));
        let request = encode_frame(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "status.get"
        }))
        .unwrap();
        round_trip(&name, &request).unwrap();
        round_trip(&name, &request).unwrap();
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_handles_bounded_concurrent_clients() {
        let name = format!(
            r"\\.\pipe\audiorouter-concurrent-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, 8, |_, frame| echo_handler(frame))
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let clients = (0..8)
            .map(|index| {
                let name = name.clone();
                std::thread::spawn(move || {
                    let request = encode_frame(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": index,
                        "method": "status.get"
                    }))
                    .unwrap();
                    let response = round_trip(&name, &request).unwrap();
                    serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap()
                })
            })
            .collect::<Vec<_>>();
        let responses = clients
            .into_iter()
            .map(|client| client.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), 8);
        assert!(responses.iter().all(|response| response["ok"] == true));
        server.join().unwrap().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn native_pipe_handles_high_volume_concurrent_clients_without_corrupting_frames() {
        const CLIENTS: usize = 32;
        let name = format!(
            r"\\.\pipe\audiorouter-high-volume-test-{}",
            std::process::id()
        );
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            serve_connections(&server_name, CLIENTS, |_, frame| echo_handler(frame))
        });
        std::thread::sleep(std::time::Duration::from_millis(20));
        let clients = (0..CLIENTS)
            .map(|index| {
                let name = name.clone();
                std::thread::spawn(move || {
                    let request = encode_frame(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": index,
                        "method": "system.describe"
                    }))
                    .unwrap();
                    let response = round_trip(&name, &request).unwrap();
                    serde_json::from_slice::<serde_json::Value>(&response[4..]).unwrap()
                })
            })
            .collect::<Vec<_>>();
        let responses = clients
            .into_iter()
            .map(|client| client.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), CLIENTS);
        for response in &responses {
            assert_eq!(response["ok"], true);
        }
        server.join().unwrap().unwrap();
    }
}
