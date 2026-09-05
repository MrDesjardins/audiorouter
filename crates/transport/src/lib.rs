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
        CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows::Win32::Security::{
        EqualSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER,
    };
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, ReadFile, WriteFile, FILE_SHARE_NONE, OPEN_EXISTING,
        PIPE_ACCESS_DUPLEX,
    };
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeClientProcessId,
        PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(once(0)).collect()
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

    fn user_sid(token: HANDLE) -> Result<windows::Win32::Security::PSID, TransportError> {
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
        Ok(user.User.Sid)
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

        let client_sid = user_sid(client_token.0)?;
        let current_sid = user_sid(current_token.0)?;
        Ok(unsafe { EqualSid(client_sid, current_sid) }.is_ok())
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

    /// Serve exactly one framed request, then disconnect and close the pipe.
    /// The default security descriptor is intentionally left visible to the caller:
    /// production callers must provide an explicit same-user ACL/authentication layer.
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
        check_name(name)?;
        let name = wide(name);
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                1,
                (MAX_FRAME_BYTES + 4) as u32,
                (MAX_FRAME_BYTES + 4) as u32,
                0,
                None,
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_invalid() {
            return Err(win_error(windows::core::Error::from_thread()));
        }
        let handle = Handle(handle);
        if let Err(error) = unsafe { ConnectNamedPipe(handle.0, None) } {
            // A client may connect between CreateNamedPipeW and ConnectNamedPipe.
            // Win32 reports that successful race as ERROR_PIPE_CONNECTED.
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
        let request = read_frame(handle.0)?;
        let response = handler(client_process_id, &request)?;
        write_all(handle.0, &response)?;
        unsafe { FlushFileBuffers(handle.0) }.map_err(win_error)?;
        let _ = unsafe { DisconnectNamedPipe(handle.0) };
        Ok(())
    }

    /// Connect to a local named pipe and exchange one framed message.
    pub fn round_trip(name: &str, request: &[u8]) -> Result<Vec<u8>, TransportError> {
        check_name(name)?;
        if request.len() < 4 || request.len() > MAX_FRAME_BYTES + 4 {
            return Err(TransportError::Protocol("invalid request frame".into()));
        }
        let name = wide(name);
        let handle = unsafe {
            CreateFileW(
                PCWSTR(name.as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                Default::default(),
                None,
            )
        }
        .map_err(win_error)?;
        let handle = Handle(handle);
        write_all(handle.0, request)?;
        read_frame(handle.0)
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
    client_is_same_user, echo_handler, round_trip, serve_once, serve_once_with_client,
};

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
pub fn client_is_same_user(_: u32) -> Result<bool, TransportError> {
    Err(TransportError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_control::ControlPlane;
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
            let mut plane = ControlPlane::new("native-test");
            serve_once(&server_name, |frame| {
                let responses = plane
                    .dispatch_frame(frame)
                    .map_err(|error| TransportError::Protocol(error.to_string()))?;
                responses.into_iter().next().ok_or_else(|| {
                    TransportError::Protocol("notification produced no response".into())
                })
            })
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
}
