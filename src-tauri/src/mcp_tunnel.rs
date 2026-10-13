//! User initiated supervisor for OpenAI's Secure MCP Tunnel runtime.
//! No tunnel is started at app launch and no remote resource is created here.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{mpsc, Mutex},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

const CONFIG_FILE: &str = "mcp-tunnel-config.dpapi";
const HEALTH_URL_FILE: &str = "mcp-tunnel-health.url";
const TUNNEL_RUNTIME_VERSION: &str = "0.0.16";

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub status: &'static str,
    pub configured: bool,
    pub tunnel_id: Option<String>,
    pub runtime_available: bool,
}

#[derive(Deserialize, Serialize)]
struct TunnelConfig {
    tunnel_id: String,
    runtime_key: String,
}

impl Drop for TunnelConfig {
    fn drop(&mut self) {
        self.runtime_key.zeroize();
    }
}

#[derive(Default)]
struct RuntimeState {
    child: Option<SupervisedChild>,
    phase: &'static str,
}

#[derive(Default)]
pub struct McpTunnelService(Mutex<RuntimeState>);

impl Drop for McpTunnelService {
    fn drop(&mut self) {
        if let Ok(state) = self.0.get_mut() {
            stop_child(&mut state.child);
        }
    }
}

fn root() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("Local app storage is unavailable.")?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return Err("Local app storage path is invalid.".into());
    }
    Ok(root.join("AudioRouter"))
}

fn check_local_path(path: &Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    if !path.is_absolute() {
        return Err("AudioRouter tunnel storage path is invalid.".into());
    }
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_attributes() & 0x400 != 0 => {
                return Err("AudioRouter tunnel storage cannot use redirected paths.".into())
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Cannot inspect AudioRouter tunnel storage.".into()),
        }
    }
    Ok(())
}

fn config_path() -> Result<PathBuf, String> {
    Ok(root()?.join(CONFIG_FILE))
}

fn health_path() -> Result<PathBuf, String> {
    Ok(root()?.join(HEALTH_URL_FILE))
}

fn valid_tunnel_id(value: &str) -> bool {
    let Some(id) = value.strip_prefix("tunnel_") else {
        return false;
    };
    let chars = |value: &str| {
        value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    };
    if id.len() == 32 {
        return chars(id);
    }
    id.len() == 37 && id.as_bytes().get(4) == Some(&b'_') && chars(&id[..4]) && chars(&id[5..])
}

fn valid_runtime_key(value: &str) -> bool {
    (20..=2048).contains(&value.len()) && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
}

fn read_config() -> Option<TunnelConfig> {
    let path = config_path().ok()?;
    check_local_path(&path).ok()?;
    let mut bytes = crate::api_token::load_secret(&path).ok()?;
    let decoded = serde_json::from_slice(&bytes);
    bytes.zeroize();
    let config: TunnelConfig = decoded.ok()?;
    (valid_tunnel_id(&config.tunnel_id) && valid_runtime_key(&config.runtime_key)).then_some(config)
}

fn save_config(tunnel_id: String, runtime_key: String) -> Result<(), String> {
    let path = config_path()?;
    check_local_path(&path)?;
    let config = TunnelConfig {
        tunnel_id,
        runtime_key,
    };
    let mut bytes = serde_json::to_vec(&config).map_err(|_| "Cannot save tunnel settings.")?;
    let result = crate::api_token::save_secret(&path, &bytes);
    bytes.zeroize();
    result
}

pub fn configure(tunnel_id: String, mut runtime_key: String) -> Result<(), String> {
    let tunnel_id = tunnel_id.trim().to_owned();
    if !valid_tunnel_id(&tunnel_id) {
        runtime_key.clear();
        return Err("Enter the tunnel ID shown on the OpenAI Platform Tunnels page.".into());
    }
    if !valid_runtime_key(&runtime_key) {
        runtime_key.zeroize();
        return Err(
            "Enter a valid restricted runtime key with Tunnels Read and Use access.".into(),
        );
    }
    save_config(tunnel_id, runtime_key)
}

pub fn status(runtime_executable: &Path) -> TunnelStatus {
    let config = read_config();
    let runtime_available = runtime_executable.is_file();
    let has_key = config.is_some();
    let mut result = TunnelStatus {
        status: "notConfigured",
        configured: config.is_some() && has_key,
        tunnel_id: config.as_ref().map(|value| value.tunnel_id.clone()),
        runtime_available,
    };
    if config.is_none() {
        return result;
    }
    if !has_key {
        result.status = "needsSetup";
        return result;
    }
    result.status = "disconnected";
    result
}

pub fn start(
    service: &McpTunnelService,
    runtime_executable: &Path,
    cli: &Path,
    database: &Path,
    pipe: &str,
) -> Result<TunnelStatus, String> {
    if !runtime_executable.is_file() {
        return Err(format!(
            "The bundled OpenAI tunnel runtime v{TUNNEL_RUNTIME_VERSION} is missing."
        ));
    }
    if !cli.is_file() || !database.is_absolute() || pipe.is_empty() {
        return Err("AudioRouter's local MCP connection is unavailable.".into());
    }
    let mut config = read_config().ok_or("Enter your OpenAI tunnel details first.")?;

    let mut state = service
        .0
        .lock()
        .map_err(|_| "Tunnel status is unavailable.")?;
    if let Some(child) = state.child.as_mut() {
        if child
            .try_wait()
            .map_err(|_| "Cannot check the tunnel process.")?
            .is_none()
        {
            return Ok(TunnelStatus {
                status: if ready() { "connected" } else { "connecting" },
                configured: true,
                tunnel_id: Some(config.tunnel_id.clone()),
                runtime_available: true,
            });
        }
    }
    stop_child(&mut state.child);

    // Read-only setup and a real backend call must work before the poller can
    // advertise readiness. The probe never receives the OpenAI credential.
    verify_local_mcp(cli, database, pipe)?;

    let health_url = health_path()?;
    check_local_path(&health_url)?;
    if health_url.exists() {
        std::fs::remove_file(&health_url).map_err(|_| "Cannot reset tunnel health status.")?;
    }
    let command = format!(
        "{} mcp serve --client-id audiorouter-chatgpt --database {}",
        quote_command(cli),
        quote_command(database)
    );
    let mut runtime_command = Command::new(runtime_executable);
    runtime_command
        .arg("run")
        .arg(format!("--control-plane.tunnel-id={}", config.tunnel_id))
        .arg("--control-plane.api-key=env:CONTROL_PLANE_API_KEY")
        .arg(format!("--mcp.command={command}"))
        .arg("--health.listen-addr=127.0.0.1:0")
        .arg(format!("--health.url-file={}", health_url.display()))
        .arg("--log.level=info")
        .env("CONTROL_PLANE_API_KEY", &config.runtime_key)
        .env_remove("OPENAI_API_KEY")
        .env("AUDIOROUTER_TUNNEL_CHILD", "1")
        .env("AUDIOROUTER_CONTROL_PIPE", pipe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let spawn_result = SupervisedChild::spawn(&mut runtime_command);
    config.runtime_key.zeroize();
    let child = spawn_result?;
    state.child = Some(child);
    state.phase = "connecting";
    Ok(TunnelStatus {
        status: state.phase,
        configured: true,
        tunnel_id: Some(config.tunnel_id.clone()),
        runtime_available: true,
    })
}

pub fn current_status(service: &McpTunnelService, runtime_executable: &Path) -> TunnelStatus {
    let mut status = status(runtime_executable);
    if !status.configured {
        return status;
    }
    let Ok(mut state) = service.0.lock() else {
        status.status = "actionNeeded";
        return status;
    };
    let Some(child) = state.child.as_mut() else {
        return status;
    };
    match child.try_wait() {
        Ok(None) => {
            status.status = if ready() {
                state.phase = "connected";
                "connected"
            } else {
                state.phase = "connecting";
                "connecting"
            };
        }
        Ok(Some(_)) => {
            state.child = None;
            state.phase = "actionNeeded";
            status.status = state.phase;
        }
        Err(_) => status.status = "actionNeeded",
    }
    status
}

pub fn disconnect(service: &McpTunnelService) -> Result<(), String> {
    let mut state = service
        .0
        .lock()
        .map_err(|_| "Tunnel status is unavailable.")?;
    stop_child(&mut state.child);
    state.phase = "disconnected";
    if let Ok(path) = health_path() {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

pub fn forget(service: &McpTunnelService) -> Result<(), String> {
    disconnect(service)?;
    let path = config_path()?;
    check_local_path(&path)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Could not remove the saved OpenAI tunnel details.".into()),
    }
}

fn quote_command(value: &Path) -> String {
    // tunnel-client's Windows stdio command parser treats backslashes as
    // escapes; its supported Windows examples use forward slashes.
    let value = value.to_string_lossy().replace('\\', "/");
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn stop_child(child: &mut Option<SupervisedChild>) {
    drop(child.take());
}

/// The job handle belongs only to the shell, never to the runtime. Windows
/// closes it even when Tauri exits without running Rust destructors, killing
/// the runtime and its stdio child together. Suspend at creation so no child
/// can escape the job before assignment.
struct SupervisedChild {
    child: Child,
    job: std::os::windows::io::OwnedHandle,
}

impl SupervisedChild {
    fn spawn(command: &mut Command) -> Result<Self, String> {
        use std::os::windows::{
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
            process::CommandExt,
        };
        use windows::{
            core::PCWSTR,
            Win32::{
                Foundation::HANDLE,
                System::{
                    JobObjects::{
                        AssignProcessToJobObject, CreateJobObjectW,
                        JobObjectExtendedLimitInformation, SetInformationJobObject,
                        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                    },
                    Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED},
                },
            },
        };
        // SAFETY: null attributes create a non-inheritable anonymous job. Its
        // successful owned handle is transferred once and closed by OwnedHandle.
        let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|_| "Cannot create tunnel process containment.")?;
        // SAFETY: CreateJobObjectW returned a unique, valid owning HANDLE.
        let job = unsafe { OwnedHandle::from_raw_handle(job.0) };
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the job lives through this call; the pointer and size refer
        // to the complete initialized limit structure, which Windows copies.
        unsafe {
            SetInformationJobObject(
                HANDLE(job.as_raw_handle()),
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            )
        }
        .map_err(|_| "Cannot configure tunnel process containment.")?;
        let child = command
            .creation_flags(CREATE_SUSPENDED.0 | CREATE_NO_WINDOW.0)
            .spawn()
            .map_err(|_| "Could not start the OpenAI tunnel runtime.")?;
        let process = Self { child, job };
        // SAFETY: both handles remain owned and valid. The new process's only
        // thread is suspended, so it cannot spawn descendants before assignment.
        unsafe {
            AssignProcessToJobObject(
                HANDLE(process.job.as_raw_handle()),
                HANDLE(process.child.as_raw_handle()),
            )
        }
        .map_err(|_| "Cannot contain the tunnel process; it was stopped.")?;
        resume_initial_thread(process.child.id())?;
        Ok(process)
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
}

impl Drop for SupervisedChild {
    fn drop(&mut self) {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{Foundation::HANDLE, System::JobObjects::TerminateJobObject};
        // SAFETY: the exclusively owned job remains valid through termination;
        // only processes this supervisor created and assigned belong to it.
        let _ = unsafe { TerminateJobObject(HANDLE(self.job.as_raw_handle()), 0) };
        // Also reap a child that could not be assigned to the job.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn resume_initial_thread(process_id: u32) -> Result<(), String> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows::Win32::{
        Foundation::HANDLE,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD,
                THREADENTRY32,
            },
            Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
        },
    };
    // SAFETY: read-only system thread snapshot; successful handle is uniquely owned.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }
        .map_err(|_| "Cannot inspect the suspended tunnel thread.")?;
    // SAFETY: snapshot is a successful owning HANDLE, transferred exactly once.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot.0) };
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    // SAFETY: valid snapshot and initialized structure with the required dwSize.
    let mut found = unsafe { Thread32First(HANDLE(snapshot.as_raw_handle()), &mut entry) }.is_ok();
    while found {
        if entry.th32OwnerProcessID == process_id {
            // SAFETY: the child is still owned and suspended; this snapshot
            // identifies its sole initial thread. Request only suspend/resume.
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) }
                .map_err(|_| "Cannot open the suspended tunnel thread.")?;
            // SAFETY: successful thread handle is uniquely owned and transferred once.
            let thread = unsafe { OwnedHandle::from_raw_handle(thread.0) };
            // SAFETY: this thread was created suspended by this supervisor;
            // the owned handle lives until the resume call returns.
            if unsafe { ResumeThread(HANDLE(thread.as_raw_handle())) } == u32::MAX {
                return Err("Cannot resume the tunnel process.".into());
            }
            return Ok(());
        }
        // SAFETY: same valid snapshot and output structure as Thread32First.
        found = unsafe { Thread32Next(HANDLE(snapshot.as_raw_handle()), &mut entry) }.is_ok();
    }
    Err("The tunnel's suspended thread was not found.".into())
}

const MCP_PROBE_INCOMPATIBLE: &str = "AudioRouter's MCP executable is outdated or incompatible. Update the CLI packaged beside the app, then Connect again.";
const MCP_PROBE_BACKEND: &str = "The MCP server cannot reach AudioRouter's backend with its current client permission. Reconnect the app and check Client permissions, then Connect again.";

fn verify_local_mcp(cli: &Path, database: &Path, pipe: &str) -> Result<(), String> {
    let mut command = Command::new(cli);
    command
        .args([
            "mcp",
            "serve",
            "--client-id",
            "audiorouter-chatgpt",
            "--database",
        ])
        .arg(database)
        .args(["--pipe", pipe])
        .env_remove("CONTROL_PLANE_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = SupervisedChild::spawn(&mut command)?;
    let mut input = child.child.stdin.take().ok_or(MCP_PROBE_INCOMPATIBLE)?;
    let output = child.child.stdout.take().ok_or(MCP_PROBE_INCOMPATIBLE)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        let mut output = BufReader::new(output);
        let result = (0..4).try_for_each(|index| {
            let mut bytes = Vec::new();
            (&mut output)
                .take(1024 * 1024 + 1)
                .read_until(b'\n', &mut bytes)
                .map_err(|_| MCP_PROBE_INCOMPATIBLE)?;
            if bytes.len() > 1024 * 1024 || bytes.last() != Some(&b'\n') {
                return Err(MCP_PROBE_INCOMPATIBLE);
            }
            let response = serde_json::from_slice(&bytes).map_err(|_| MCP_PROBE_INCOMPATIBLE)?;
            validate_probe_response(index, &response)
        });
        let _ = sender.send(result);
    });
    let requests = [
        serde_json::json!({"jsonrpc":"2.0","id":"discover","method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}),
        serde_json::json!({"jsonrpc":"2.0","id":"initialize","method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"audiorouter-preflight","version":"1"},"_meta":{"progressToken":"preflight"}}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        serde_json::json!({"jsonrpc":"2.0","id":"tools","method":"tools/list","params":{}}),
        serde_json::json!({"jsonrpc":"2.0","id":"backend","method":"tools/call","params":{"name":"describe_capabilities","arguments":{}}}),
    ];
    let sent = requests
        .iter()
        .try_for_each(|request| writeln!(input, "{request}"));
    let result = if sent.is_err() || input.flush().is_err() {
        Err(MCP_PROBE_INCOMPATIBLE)
    } else {
        receiver.recv_timeout(Duration::from_secs(5)).unwrap_or(Err("The local MCP server did not answer within five seconds. Update the app or reconnect its backend, then try again."))
    };
    drop(input);
    drop(child); // kills stalled probes and closes stdout before joining its reader
    let _ = reader.join();
    result.map_err(str::to_owned)
}

fn validate_probe_response(index: usize, response: &serde_json::Value) -> Result<(), &'static str> {
    let expected_id = ["discover", "initialize", "tools", "backend"][index];
    if response["jsonrpc"] != "2.0"
        || response["id"] != expected_id
        || response.get("error").is_some()
    {
        return Err(if index == 3 {
            MCP_PROBE_BACKEND
        } else {
            MCP_PROBE_INCOMPATIBLE
        });
    }
    let valid = match index {
        0 => response["result"]["supportedVersions"]
            .as_array()
            .is_some_and(|versions| versions.iter().any(|version| version == "2026-07-28")),
        1 => response["result"]["protocolVersion"] == "2025-11-25",
        2 => response["result"]["tools"].as_array().is_some_and(|tools| {
            tools
                .iter()
                .any(|tool| tool["name"] == "describe_capabilities")
        }),
        3 => {
            response["result"]["isError"] == false
                && response["result"]["content"]
                    .as_array()
                    .is_some_and(|content| !content.is_empty())
        }
        _ => false,
    };
    valid.then_some(()).ok_or(if index == 3 {
        MCP_PROBE_BACKEND
    } else {
        MCP_PROBE_INCOMPATIBLE
    })
}

fn ready() -> bool {
    let Ok(path) = health_path() else {
        return false;
    };
    let Ok(metadata) = std::fs::symlink_metadata(&path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 512 {
        return false;
    }
    let Ok(url) = std::fs::read_to_string(path) else {
        return false;
    };
    let Some(address) = loopback_health_address(url.trim()) else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(250)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    if stream
        .write_all(b"GET /readyz HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut response = [0u8; 256];
    let Ok(count) = stream.read(&mut response) else {
        return false;
    };
    response[..count].starts_with(b"HTTP/1.1 200") || response[..count].starts_with(b"HTTP/1.0 200")
}

fn loopback_health_address(url: &str) -> Option<SocketAddr> {
    let address = url.strip_prefix("http://127.0.0.1:")?;
    let port = address.split('/').next()?.parse::<u16>().ok()?;
    (port != 0).then_some(SocketAddr::from(([127, 0, 0, 1], port)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_failures_are_actionable_and_never_echo_server_errors() {
        let secret_error = serde_json::json!({"jsonrpc":"2.0","id":"discover","error":{"message":"private-token-sentinel"}});
        assert_eq!(
            validate_probe_response(0, &secret_error),
            Err(MCP_PROBE_INCOMPATIBLE)
        );
        let backend_error = serde_json::json!({"jsonrpc":"2.0","id":"backend","result":{"isError":true,"content":[{"text":"private-path-sentinel"}]}});
        assert_eq!(
            validate_probe_response(3, &backend_error),
            Err(MCP_PROBE_BACKEND)
        );
        assert!(!MCP_PROBE_BACKEND.contains("sentinel"));
    }

    #[test]
    #[ignore = "Read-only live host MCP probe; requires explicit CLI/database/pipe environment"]
    fn live_mcp_sidecar_preflight() {
        let cli = PathBuf::from(std::env::var_os("AUDIOROUTER_MCP_SMOKE_CLI").expect("CLI"));
        let database =
            PathBuf::from(std::env::var_os("AUDIOROUTER_MCP_SMOKE_DATABASE").expect("database"));
        let pipe = std::env::var("AUDIOROUTER_MCP_SMOKE_PIPE").expect("pipe");
        let result = verify_local_mcp(&cli, &database, &pipe);
        if std::env::var_os("AUDIOROUTER_MCP_SMOKE_EXPECT_STALE").is_some() {
            assert_eq!(result, Err(MCP_PROBE_INCOMPATIBLE.to_owned()));
        } else {
            result.expect("modern discovery, legacy initialize, tools/list and backend read");
        }
    }

    fn descendant_fixture() -> Command {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-Command", "$p = Start-Process powershell.exe -ArgumentList '-NoProfile','-Command','Start-Sleep -Seconds 120' -WindowStyle Hidden -PassThru; [Console]::Out.WriteLine($p.Id); Start-Sleep -Seconds 120"])
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
        command
    }

    #[test]
    fn job_owner_exit_fixture() {
        if std::env::var_os("AUDIOROUTER_JOB_EXIT_FIXTURE").is_none() {
            return;
        }
        let mut process =
            SupervisedChild::spawn(&mut descendant_fixture()).expect("fixture process");
        let output = process.child.stdout.take().unwrap();
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut pid = String::new();
            BufReader::new(output).read_line(&mut pid).unwrap();
            let _ = sender.send(pid.trim().parse::<u32>().unwrap());
        });
        let descendant = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("descendant PID");
        println!("JOB_FIXTURE:{},{}", process.child.id(), descendant);
        std::io::stdout().flush().unwrap();
        // Deliberately skip Drop, as Tauri process exit/forced termination can.
        std::process::exit(0);
    }

    #[test]
    fn shell_exit_stops_runtime_and_its_descendant_without_destructors() {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
        use windows::Win32::{
            Foundation::{HANDLE, WAIT_OBJECT_0},
            System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
        };
        let mut fixture = Command::new(std::env::current_exe().unwrap());
        fixture
            .args([
                "--exact",
                "mcp_tunnel::tests::job_owner_exit_fixture",
                "--nocapture",
            ])
            .env("AUDIOROUTER_JOB_EXIT_FIXTURE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = fixture.output().expect("launch job owner");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        let ids = text
            .lines()
            .find_map(|line| line.strip_prefix("JOB_FIXTURE:"))
            .expect("runtime and descendant PID");
        for id in ids.split(',').map(|id| id.parse::<u32>().unwrap()) {
            // SAFETY: request wait-only access to the fixture's recorded PID.
            // A process that has already disappeared is already stopped.
            if let Ok(handle) = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, id) } {
                // SAFETY: successful uniquely owned process handle, transferred once.
                let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
                // SAFETY: handle remains live until the bounded wait returns.
                let stopped = unsafe { WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000) };
                assert_eq!(stopped, WAIT_OBJECT_0, "process {id} survived owner exit");
            }
        }
    }

    #[test]
    fn accepts_only_supported_tunnel_ids_and_runtime_keys() {
        assert!(valid_tunnel_id("tunnel_0123456789abcdef0123456789abcdef"));
        assert!(valid_tunnel_id(
            "tunnel_ab12_0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_tunnel_id(
            "https://example.test/tunnel_0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_tunnel_id("tunnel_0123456789ABCDEF0123456789abcdef"));
        assert!(!valid_runtime_key("short"));
        assert!(!valid_runtime_key(&"x".repeat(20).replace('x', "\n")));
        assert!(valid_runtime_key(&"k".repeat(64)));
    }

    #[test]
    fn readiness_url_is_loopback_only() {
        assert_eq!(
            loopback_health_address("http://127.0.0.1:1234"),
            Some(([127, 0, 0, 1], 1234).into())
        );
        assert!(loopback_health_address("http://localhost:1234").is_none());
        assert!(loopback_health_address("http://192.168.1.2:1234").is_none());
        assert!(loopback_health_address("https://127.0.0.1:1234").is_none());
    }
}
